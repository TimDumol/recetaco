package dev.recetaco.app

import kotlinx.coroutines.runBlocking
import okhttp3.mockwebserver.MockResponse
import okhttp3.mockwebserver.MockWebServer
import org.junit.Assert.*
import org.junit.Test

class ApiTest {
    @Test
    fun `cook logs preserve date rating and reflection notes`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(
                MockResponse()
                    .setBody(
                        """{"id":9,"recipe_id":7,"cooked_on":"2026-10-24","rating":4,"cook_name":"Both","went_well":"Texture","to_improve":"Less salt","notes":"Used stock"}"""
                    )
            )
            val api = RecetacoApi(server.url("/").toString(), "token")
            val saved =
                api.saveCook(
                    7,
                    Cook(
                        cooked_on = "2026-10-24",
                        rating = 4,
                        cook_name = "Both",
                        went_well = "Texture",
                        to_improve = "Less salt",
                        notes = "Used stock",
                    ),
                )
            assertEquals(9L, saved.id)
            assertEquals("Less salt", saved.to_improve)
            val request = server.takeRequest()
            assertEquals("POST", request.method)
            assertEquals("/api/recipes/7/cooks", request.path)
            val payload = RecetacoApi.json.decodeFromString<Cook>(request.body.readUtf8())
            assertEquals(4, payload.rating)
            assertEquals("Texture", payload.went_well)
        }
    }

    @Test
    fun `existing cook edits use PUT and keep their identity`() = runBlocking {
        MockWebServer().use { server ->
            val cook =
                Cook(
                    id = 9,
                    recipe_id = 7,
                    cooked_on = "2026-10-24",
                    rating = 2,
                    notes = "Too soft",
                )
            server.enqueue(
                MockResponse().setBody(RecetacoApi.json.encodeToString(Cook.serializer(), cook))
            )
            RecetacoApi(server.url("/").toString(), "").saveCook(7, cook)
            val request = server.takeRequest()
            assertEquals("PUT", request.method)
            assertEquals("/api/recipes/7/cooks/9", request.path)
        }
    }

    @Test
    fun `clearing shared recipe rating does not create a cook`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(
                MockResponse()
                    .setBody(
                        """{"id":7,"title":"Rice","servings":2,"ingredients":[],"rating":null,"cook_count":2,"average_cook_rating":3.5,"last_cooked":"2026-10-24"}"""
                    )
            )
            val recipe = RecetacoApi(server.url("/").toString(), "").rateRecipe(7, null)
            assertNull(recipe.rating)
            assertEquals(2L, recipe.cook_count)
            assertEquals(3.5, recipe.average_cook_rating!!, 0.001)
            val request = server.takeRequest()
            assertEquals("/api/recipes/7/rating", request.path)
            assertEquals("PUT", request.method)
            assertEquals("{\"rating\":null}", request.body.readUtf8())
        }
    }

    @Test
    fun `weekly wishlist decodes without requiring a meal date`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(
                MockResponse()
                    .setBody(
                        """[{"id":1,"recipe_id":4,"title":"Rice","week":"2026-11-02","servings":4,"cooked":false}]"""
                    )
            )
            val wishes = RecetacoApi(server.url("/").toString(), "").wishes("2026-11-02")
            assertEquals("2026-11-02", wishes[0].week)
            assertFalse(wishes[0].cooked)
            assertEquals("/api/wishes?week=2026-11-02", server.takeRequest().path)
        }
    }

    @Test
    fun `whole week shopping uses the selected week`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(MockResponse().setBody("{}"))
            RecetacoApi(server.url("/").toString(), "").shopWeek(3, "2026-11-02")
            val request = server.takeRequest()
            assertEquals("POST", request.method)
            assertEquals("/api/lists/3/weeks/2026-11-02", request.path)
        }
    }

    @Test
    fun `photo upload uses binary JPEG with the current access token`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(MockResponse().setBody("{}"))
            val bytes = byteArrayOf(1, 2, 3, 4)
            RecetacoApi(server.url("/").toString(), "", tokenProvider = { "current-token" })
                .uploadPhoto(7, bytes)
            val request = server.takeRequest()
            assertEquals("PUT", request.method)
            assertEquals("/api/recipes/7/photo", request.path)
            assertEquals("Bearer current-token", request.getHeader("Authorization"))
            assertEquals("image/jpeg", request.getHeader("Content-Type"))
            assertArrayEquals(bytes, request.body.readByteArray())
        }
    }

    @Test
    fun `API uses refreshed access token instead of a stored credential`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(MockResponse().setBody("[]"))
            val api =
                RecetacoApi(
                    server.url("/").toString(),
                    "old-token",
                    tokenProvider = { "fresh-pocket-id-token" },
                )
            api.recipes()
            assertEquals(
                "Bearer fresh-pocket-id-token",
                server.takeRequest().getHeader("Authorization"),
            )
        }
    }

    @Test
    fun `Pocket ID configuration matches Rust public discovery payload`() {
        val config =
            RecetacoApi.json.decodeFromString<LoginConfig>(
                """{"mode":"oidc","oidc":{"issuer":"https://id.example","client_id":"web","android_client_id":"android","audience":"https://recetaco.example"}}"""
            )
        assertEquals("android", config.oidc?.android_client_id)
        assertEquals("https://recetaco.example", config.oidc?.audience)
    }

    @Test
    fun `recipes decode Rust JSON and requests carry authentication`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(
                MockResponse()
                    .setBody(
                        """[{"id":1,"title":"Rice","servings":2,"instructions":"Cook","source_url":"","ingredients":[{"name":"Rice","quantity":100,"unit":"g"}]}]"""
                    )
            )
            val api = RecetacoApi(server.url("/").toString(), "test-token")
            val recipes = api.recipes()
            assertEquals("Rice", recipes[0].title)
            assertEquals(100.0, recipes[0].ingredients[0].quantity, 0.001)
            val request = server.takeRequest()
            assertEquals("/api/recipes", request.path)
            assertEquals("Bearer test-token", request.getHeader("Authorization"))
        }
    }

    @Test
    fun `adding a recipe sends selected serving count`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(MockResponse().setBody("{}"))
            RecetacoApi(server.url("/").toString(), "").addRecipe(3, 7, 4.0)
            val request = server.takeRequest()
            assertEquals("POST", request.method)
            assertEquals("/api/lists/3/recipes/7", request.path)
            assertEquals("{\"servings\":4.0}", request.body.readUtf8())
        }
    }

    @Test
    fun `server validation errors remain understandable`() = runBlocking {
        MockWebServer().use { server ->
            server.enqueue(
                MockResponse()
                    .setResponseCode(400)
                    .setBody("""{"error":"Quantity must be positive"}""")
            )
            val error =
                runCatching { RecetacoApi(server.url("/").toString(), "").recipes() }
                    .exceptionOrNull()
            assertEquals("Quantity must be positive", error?.message)
        }
    }

    @Test
    fun `server origin cannot hide a query or embedded credentials`() {
        for (url in
            listOf(
                "not a URL",
                "http://recetaco.example/",
                "http://192.168.1.10/",
                "http://127.0.0.1.attacker.example/",
                "https://host/api",
                "https://host/?token=x",
                "https://user:pass@host/",
            )) {
            assertTrue(runCatching { RecetacoApi(url, "") }.isFailure)
        }
    }
}
