package dev.recetaco.app

import android.content.Context
import android.net.Uri
import java.util.concurrent.TimeUnit
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import net.openid.appauth.AuthState
import net.openid.appauth.AuthorizationRequest
import net.openid.appauth.AuthorizationService
import net.openid.appauth.AuthorizationServiceConfiguration
import net.openid.appauth.ResponseTypeValues
import okhttp3.OkHttpClient
import okhttp3.Request

@Serializable data class LoginConfig(val mode: String, val oidc: ProviderConfig? = null)

@Serializable
data class ProviderConfig(
    val issuer: String,
    val client_id: String,
    val android_client_id: String,
    val audience: String,
)

suspend fun loginConfig(url: String): LoginConfig =
    withContext(Dispatchers.IO) {
        // Reuse origin validation before any network access.
        RecetacoApi(url, "")
        val client =
            OkHttpClient.Builder().callTimeout(15, TimeUnit.SECONDS).followRedirects(false).build()
        client
            .newCall(Request.Builder().url(url.trimEnd('/') + "/auth/config").build())
            .execute()
            .use {
                check(it.isSuccessful) { "Could not load this server's sign-in configuration." }
                RecetacoApi.json.decodeFromString(
                    it.body?.string() ?: error("Empty sign-in configuration")
                )
            }
    }

suspend fun authorizationRequest(config: ProviderConfig): AuthorizationRequest {
    require(config.issuer.startsWith("https://")) { "Pocket ID sign-in requires HTTPS." }
    val metadata =
        suspendCancellableCoroutine<AuthorizationServiceConfiguration> { cont ->
            AuthorizationServiceConfiguration.fetchFromIssuer(Uri.parse(config.issuer)) {
                service,
                error ->
                if (cont.isActive) {
                    if (service != null) cont.resume(service)
                    else
                        cont.resumeWithException(
                            error ?: IllegalStateException("Pocket ID discovery failed.")
                        )
                }
            }
        }
    return AuthorizationRequest.Builder(
            metadata,
            config.android_client_id,
            ResponseTypeValues.CODE,
            Uri.parse("dev.recetaco.app:/oauth2redirect"),
        )
        .setScopes("openid", "profile", "offline_access")
        .setAdditionalParameters(mapOf("resource" to config.audience))
        .build() // AppAuth generates state, nonce, and an S256 PKCE verifier.
}

/** AppAuth refreshes access tokens, while the full authorization state stays encrypted. */
class OidcSession(
    context: Context,
    private val url: String,
    val state: AuthState,
    private val connection: ConnectionStore,
) {
    private val service = AuthorizationService(context)

    suspend fun accessToken(): String = suspendCancellableCoroutine { cont ->
        val resource =
            state.lastAuthorizationResponse?.request?.additionalParameters?.get("resource")
        val refreshParameters = resource?.let { mapOf("resource" to it) } ?: emptyMap()
        state.performActionWithFreshTokens(service, refreshParameters) { token, _, error ->
            if (cont.isActive) {
                if (error != null || token == null)
                    cont.resumeWithException(
                        error ?: IllegalStateException("Sign in with Pocket ID again.")
                    )
                else
                    try {
                        connection.save(url, state.jsonSerializeString())
                        cont.resume(token)
                    } catch (e: Exception) {
                        cont.resumeWithException(e)
                    }
            }
        }
    }

    fun close() = service.dispose()
}
