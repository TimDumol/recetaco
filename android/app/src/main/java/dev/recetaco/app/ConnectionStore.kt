package dev.recetaco.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** API tokens stay encrypted at rest with a non-exportable Android Keystore key. */
class ConnectionStore(context: Context) {
    private val prefs = context.getSharedPreferences("connection", Context.MODE_PRIVATE)
    val url: String
        get() = prefs.getString("url", "").orEmpty()

    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey("recetaco-api-token", null) as? SecretKey)?.let {
            return it
        }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
            .apply {
                init(
                    KeyGenParameterSpec.Builder(
                            "recetaco-api-token",
                            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
                        )
                        .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                        .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                        .build()
                )
            }
            .generateKey()
    }

    val token: String
        get() =
            runCatching {
                    val raw = prefs.getString("token", null) ?: return@runCatching ""
                    val bytes = Base64.decode(raw, Base64.NO_WRAP)
                    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                    cipher.init(
                        Cipher.DECRYPT_MODE,
                        key(),
                        GCMParameterSpec(128, bytes.copyOfRange(0, 12)),
                    )
                    String(cipher.doFinal(bytes.copyOfRange(12, bytes.size)), Charsets.UTF_8)
                }
                .getOrDefault("")

    fun save(url: String, token: String) {
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val encrypted = cipher.iv + cipher.doFinal(token.toByteArray(Charsets.UTF_8))
        check(
            prefs
                .edit()
                .putString("url", url)
                .putString("token", Base64.encodeToString(encrypted, Base64.NO_WRAP))
                .commit()
        ) {
            "Could not save connection settings"
        }
    }
}
