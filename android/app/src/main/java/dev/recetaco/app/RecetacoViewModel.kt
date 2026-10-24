package dev.recetaco.app

import android.app.Application
import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import java.time.DayOfWeek
import java.time.LocalDate
import java.time.temporal.TemporalAdjusters
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.launch
import net.openid.appauth.AuthState

class RecetacoViewModel(application: Application) : AndroidViewModel(application) {
    private val connection = ConnectionStore(application)
    var serverUrl by mutableStateOf(connection.url)
        private set

    private var oidcSession: OidcSession? =
        runCatching {
                OidcSession(
                    application,
                    serverUrl,
                    AuthState.jsonDeserialize(connection.token),
                    connection,
                )
            }
            .getOrNull()
    var token by mutableStateOf(if (oidcSession == null) connection.token else "")
        private set

    private var api: RecetacoApi? = runCatching { makeApi(serverUrl, token) }.getOrNull()

    private fun makeApi(url: String, token: String): RecetacoApi =
        RecetacoApi(
            url,
            token,
            tokenProvider = oidcSession?.let { session -> { session.accessToken() } },
        )

    fun completeSignIn(url: String, state: AuthState, done: () -> Unit) {
        operation {
            val accessToken =
                state.accessToken ?: error("Pocket ID did not return an access token.")
            load(RecetacoApi(url, accessToken))
            val session = OidcSession(getApplication(), url, state, connection)
            try {
                connection.save(url, state.jsonSerializeString())
            } catch (e: Exception) {
                session.close()
                throw e
            }
            oidcSession?.close()
            oidcSession = session
            api = RecetacoApi(url, "", tokenProvider = { session.accessToken() })
            serverUrl = url
            token = ""
            done()
        }
    }

    fun signOut() {
        oidcSession?.close()
        oidcSession = null
        connection.save(serverUrl, "")
        token = ""
        loaded = false
        recipes = emptyList()
        cooks = emptyList()
        historyRecipeId = null
        meals = emptyList()
        wishes = emptyList()
        lists = emptyList()
        detail = null
        api = runCatching { RecetacoApi(serverUrl, "") }.getOrNull()
    }

    override fun onCleared() {
        oidcSession?.close()
        super.onCleared()
    }

    var cooks by mutableStateOf<List<Cook>>(emptyList())
        private set

    var historyRecipeId by mutableStateOf<Long?>(null)
        private set

    var recipes by mutableStateOf<List<Recipe>>(emptyList())
        private set

    var week by
        mutableStateOf(
            LocalDate.now().with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY)).toString()
        )
        private set

    var wishes by mutableStateOf<List<Wish>>(emptyList())
        private set

    var meals by mutableStateOf<List<Meal>>(emptyList())
        private set

    var lists by mutableStateOf<List<ShoppingList>>(emptyList())
        private set

    var detail by mutableStateOf<ListDetail?>(null)
        private set

    var busy by mutableStateOf(false)
        private set

    var error by mutableStateOf<String?>(null)
        private set

    var loaded by mutableStateOf(false)
        private set

    init {
        if (api != null) refresh()
    }

    private fun action(block: suspend (RecetacoApi) -> Unit) = operation {
        block(api ?: error("Connect to your Recetaco server first."))
    }

    private fun operation(block: suspend () -> Unit) {
        if (busy) return
        viewModelScope.launch {
            busy = true
            error = null
            try {
                block()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: "Could not reach your server."
            } finally {
                busy = false
            }
        }
    }

    private suspend fun load(client: RecetacoApi) {
        val r = client.recipes()
        val m = client.meals()
        val l = client.lists()
        val w = client.wishes(week)
        val d = l.firstOrNull { it.id == detail?.id } ?: l.firstOrNull()
        val newDetail = d?.let { client.list(it.id) }
        recipes = r
        meals = m
        lists = l
        wishes = w
        detail = newDetail
        loaded = true
    }

    fun refresh() = action { load(it) }

    fun connect(url: String, token: String, done: () -> Unit) {
        if (busy) return
        viewModelScope.launch {
            busy = true
            error = null
            try {
                val normalized = url.trim().trimEnd('/')
                val client =
                    if (normalized == serverUrl && oidcSession != null) makeApi(normalized, "")
                    else RecetacoApi(normalized, token.trim())
                // Fetch successfully before replacing the working connection.
                val r = client.recipes()
                val m = client.meals()
                val l = client.lists()
                val w = client.wishes(week)
                val d = l.firstOrNull()?.let { client.list(it.id) }
                if (normalized != serverUrl || oidcSession == null) {
                    oidcSession?.close()
                    oidcSession = null
                    connection.save(client.baseUrl, token.trim())
                }
                api = client
                serverUrl = client.baseUrl
                this@RecetacoViewModel.token = token.trim()
                recipes = r
                meals = m
                lists = l
                wishes = w
                detail = d
                loaded = true
                done()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                error = e.message ?: "Connection failed."
            } finally {
                busy = false
            }
        }
    }

    fun save(
        recipe: Recipe,
        photo: Uri?,
        removePhoto: Boolean,
        persisted: (Recipe) -> Unit,
        done: () -> Unit,
    ) = action { client ->
        val saved = client.save(recipe)
        recipes = recipes.filter { it.id != saved.id } + saved
        persisted(saved)
        if (photo != null) client.uploadPhoto(saved.id, preparePhoto(getApplication(), photo))
        else if (removePhoto) client.delete("/recipes/${saved.id}/photo")
        load(client)
        done()
    }

    suspend fun photo(id: Long): ByteArray = api?.photo(id) ?: error("Not connected")

    fun changeWeek(value: String) = action { client ->
        val target =
            LocalDate.parse(value)
                .with(TemporalAdjusters.previousOrSame(DayOfWeek.MONDAY))
                .toString()
        val result = client.wishes(target)
        week = target
        wishes = result
    }

    fun loadCooks(id: Long) = action { client ->
        if (historyRecipeId != id) cooks = emptyList()
        historyRecipeId = id
        cooks = client.cooks(id)
    }

    fun rateRecipe(id: Long, rating: Int?) = action {
        val updated = it.rateRecipe(id, rating)
        recipes = recipes.map { r -> if (r.id == id) updated else r }
    }

    fun saveCook(id: Long, data: Cook, persisted: (Cook) -> Unit, done: () -> Unit) =
        action { client ->
            val saved = client.saveCook(id, data)
            cooks = cooks.filter { it.id != saved.id } + saved
            persisted(saved)
            load(client)
            cooks = client.cooks(id)
            historyRecipeId = id
            done()
        }

    fun deleteCook(cook: Cook) = action { client ->
        client.delete("/recipes/${cook.recipe_id}/cooks/${cook.id}")
        cooks = cooks.filter { it.id != cook.id }
        load(client)
        cooks = client.cooks(cook.recipe_id)
        historyRecipeId = cook.recipe_id
    }

    fun wish(data: Wish, done: () -> Unit) = action {
        it.wish(data)
        load(it)
        done()
    }

    fun cook(wish: Wish) = action {
        it.cook(wish)
        load(it)
    }

    fun deleteWish(wish: Wish) = action {
        it.delete("/wishes/${wish.id}")
        load(it)
    }

    fun shopWeek(listId: Long?, name: String, done: () -> Unit) = action {
        val id = listId ?: it.createList(name).id
        it.shopWeek(id, week)
        load(it)
        detail = it.list(id)
        done()
    }

    fun deleteRecipe(recipe: Recipe, done: () -> Unit) = action {
        it.delete("/recipes/${recipe.id}")
        load(it)
        done()
    }

    fun plan(meal: Meal, done: () -> Unit) = action {
        it.plan(meal)
        load(it)
        done()
    }

    fun deleteMeal(meal: Meal) = action {
        it.delete("/meals/${meal.id}")
        load(it)
    }

    fun selectList(id: Long) = action { detail = it.list(id) }

    fun createList(name: String, done: () -> Unit) = action {
        val list = it.createList(name)
        load(it)
        detail = it.list(list.id)
        done()
    }

    fun deleteList(id: Long, done: () -> Unit) = action {
        it.delete("/lists/$id")
        load(it)
        done()
    }

    fun addRecipe(
        listId: Long?,
        listName: String,
        recipe: Recipe,
        servings: Double,
        done: () -> Unit,
    ) = action {
        val id = listId ?: it.createList(listName).id
        it.addRecipe(id, recipe.id, servings)
        load(it)
        detail = it.list(id)
        done()
    }

    fun addItem(item: Ingredient, done: () -> Unit) = action { client ->
        val id = detail?.id ?: error("Select a list first.")
        client.addItem(id, item)
        detail = client.list(id)
        done()
    }

    fun check(item: ShoppingItem) = action {
        it.check(item.list_id, item)
        detail = it.list(item.list_id)
    }

    fun deleteItem(item: ShoppingItem) = action {
        it.delete("/lists/${item.list_id}/items/${item.id}")
        detail = it.list(item.list_id)
    }

    val signedInWithPocketId: Boolean
        get() = oidcSession != null

    fun authenticationError(message: String) {
        error = message
    }

    fun clearError() {
        error = null
    }
}
