package dev.recetaco.app

import java.util.concurrent.TimeUnit
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody

@Serializable
data class Ingredient(val name: String = "", val quantity: Double = 1.0, val unit: String = "")

@Serializable
data class Recipe(
    val id: Long = 0,
    val title: String = "",
    val servings: Double = 2.0,
    val instructions: String = "",
    val source_url: String = "",
    val ingredients: List<Ingredient> = listOf(Ingredient()),
    val photo_version: Long = 0,
    val rating: Int? = null,
    val cook_count: Long = 0,
    val average_cook_rating: Double? = null,
    val last_cooked: String? = null,
)

@Serializable
data class Cook(
    val id: Long = 0,
    val recipe_id: Long = 0,
    val cooked_on: String,
    val rating: Int? = null,
    val cook_name: String = "",
    val went_well: String = "",
    val to_improve: String = "",
    val notes: String = "",
)

@Serializable data class RatingInput(val rating: Int?)

@Serializable
data class Meal(
    val id: Long = 0,
    val recipe_id: Long,
    val title: String = "",
    val date: String,
    val servings: Double,
    val slot: String = "dinner",
)

@Serializable
data class Wish(
    val id: Long = 0,
    val recipe_id: Long,
    val title: String = "",
    val week: String,
    val servings: Double,
    val cooked: Boolean = false,
)

@Serializable data class CookedInput(val cooked: Boolean)

@Serializable data class ShoppingList(val id: Long, val name: String)

@Serializable
data class ShoppingItem(
    val id: Long,
    val list_id: Long,
    val name: String,
    val quantity: Double,
    val unit: String,
    val checked: Boolean,
)

@Serializable data class ListDetail(val id: Long, val name: String, val items: List<ShoppingItem>)

@Serializable data class ListInput(val name: String)

@Serializable data class CheckInput(val checked: Boolean)

@Serializable data class ServingsInput(val servings: Double)

class RecetacoApi(
    val baseUrl: String,
    private val token: String,
    private val client: OkHttpClient =
        OkHttpClient.Builder()
            .callTimeout(20, TimeUnit.SECONDS)
            .followSslRedirects(false)
            .build(),
    private val tokenProvider: (suspend () -> String)? = null,
) {
    companion object {
        val json = Json {
            ignoreUnknownKeys = true
            encodeDefaults = true
        }
    }

    init {
        val url = baseUrl.toHttpUrlOrNull()
        require(
            url != null &&
                (url.isHttps || url.host in setOf("localhost", "127.0.0.1", "::1")) &&
                url.encodedPath == "/" &&
                url.query == null &&
                url.fragment == null &&
                url.username.isEmpty() &&
                url.password.isEmpty()
        ) {
            "Use an HTTPS server origin (HTTP is allowed only on loopback for development)"
        }
    }

    suspend fun request(path: String, method: String = "GET", body: String? = null): String =
        withContext(Dispatchers.IO) {
            val builder = Request.Builder().url(baseUrl.trimEnd('/') + "/api" + path)
            val credential = tokenProvider?.invoke() ?: token
            if (credential.isNotEmpty()) builder.header("Authorization", "Bearer $credential")
            val requestBody = body?.toRequestBody("application/json".toMediaType())
            builder.method(method, requestBody)
            client.newCall(builder.build()).execute().use { response ->
                val text = response.body?.string().orEmpty()
                if (!response.isSuccessful) {
                    val detail =
                        runCatching {
                                json
                                    .parseToJsonElement(text)
                                    .jsonObject["error"]
                                    ?.jsonPrimitive
                                    ?.content
                            }
                            .getOrNull()
                    throw IllegalStateException(
                        if (response.code == 401)
                            "Your session expired. Sign in again through Connection settings."
                        else detail ?: text.ifBlank { "Server returned ${response.code}" }
                    )
                }
                text
            }
        }

    suspend fun recipes(): List<Recipe> = json.decodeFromString(request("/recipes"))

    suspend fun meals(): List<Meal> = json.decodeFromString(request("/meals"))

    suspend fun lists(): List<ShoppingList> = json.decodeFromString(request("/lists"))

    suspend fun list(id: Long): ListDetail = json.decodeFromString(request("/lists/$id"))

    suspend fun save(recipe: Recipe): Recipe =
        json.decodeFromString(
            request(
                "/recipes" + if (recipe.id > 0) "/${recipe.id}" else "",
                if (recipe.id > 0) "PUT" else "POST",
                json.encodeToString(recipe),
            )
        )

    suspend fun createList(name: String): ShoppingList =
        json.decodeFromString(request("/lists", "POST", json.encodeToString(ListInput(name))))

    suspend fun plan(meal: Meal) {
        request("/meals", "POST", json.encodeToString(meal))
    }

    suspend fun addRecipe(listId: Long, recipeId: Long, servings: Double) {
        request(
            "/lists/$listId/recipes/$recipeId",
            "POST",
            json.encodeToString(ServingsInput(servings)),
        )
    }

    suspend fun addItem(listId: Long, item: Ingredient) {
        request("/lists/$listId/items", "POST", json.encodeToString(item))
    }

    suspend fun check(listId: Long, item: ShoppingItem) {
        request(
            "/lists/$listId/items/${item.id}",
            "PATCH",
            json.encodeToString(CheckInput(!item.checked)),
        )
    }

    suspend fun delete(path: String) {
        request(path, "DELETE")
    }

    suspend fun wishes(week: String): List<Wish> =
        json.decodeFromString(request("/wishes?week=$week"))

    suspend fun wish(data: Wish) {
        request("/wishes", "POST", json.encodeToString(data))
    }

    suspend fun cook(wish: Wish) {
        request("/wishes/${wish.id}", "PATCH", json.encodeToString(CookedInput(!wish.cooked)))
    }

    suspend fun shopWeek(listId: Long, week: String) {
        request("/lists/$listId/weeks/$week", "POST", "{}")
    }

    suspend fun photo(id: Long): ByteArray = binary("/recipes/$id/photo")

    suspend fun uploadPhoto(id: Long, bytes: ByteArray) {
        binary("/recipes/$id/photo", bytes)
    }

    private suspend fun binary(path: String, bytes: ByteArray? = null): ByteArray =
        withContext(Dispatchers.IO) {
            val builder = Request.Builder().url(baseUrl.trimEnd('/') + "/api" + path)
            val credential = tokenProvider?.invoke() ?: token
            if (credential.isNotBlank()) builder.header("Authorization", "Bearer $credential")
            if (bytes != null) builder.put(bytes.toRequestBody("image/jpeg".toMediaType()))
            client.newCall(builder.build()).execute().use { response ->
                check(response.isSuccessful) {
                    "Photo request failed (${response.code}). Your recipe is saved; reopen its editor to retry the photo."
                }
                response.body?.bytes() ?: ByteArray(0)
            }
        }

    suspend fun cooks(recipeId: Long): List<Cook> =
        json.decodeFromString(request("/recipes/$recipeId/cooks"))

    suspend fun rateRecipe(recipeId: Long, rating: Int?): Recipe =
        json.decodeFromString(
            request("/recipes/$recipeId/rating", "PUT", json.encodeToString(RatingInput(rating)))
        )

    suspend fun saveCook(recipeId: Long, cook: Cook): Cook =
        json.decodeFromString(
            request(
                "/recipes/$recipeId/cooks" + if (cook.id > 0) "/${cook.id}" else "",
                if (cook.id > 0) "PUT" else "POST",
                json.encodeToString(cook),
            )
        )
}
