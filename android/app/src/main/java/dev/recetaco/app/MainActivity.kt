package dev.recetaco.app

import android.net.Uri
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.outlined.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.viewmodel.compose.viewModel
import java.time.LocalDate
import java.util.Locale
import kotlinx.coroutines.launch
import net.openid.appauth.AuthState
import net.openid.appauth.AuthorizationResponse
import net.openid.appauth.AuthorizationService

private val cream = Color(0xFFF8F7F2)
private val green = Color(0xFF324D3D)
private val orange = Color(0xFFB95131)
private val colors =
    lightColorScheme(
        primary = orange,
        secondary = green,
        background = cream,
        surface = cream,
        surfaceVariant = Color(0xFFE9EDDF),
        onSurface = Color(0xFF28352D),
    )

fun qty(n: Double): String = String.format(Locale.US, "%.3f", n).trimEnd('0').trimEnd('.')

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent { MaterialTheme(colorScheme = colors) { RecetacoApp() } }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RecetacoApp(vm: RecetacoViewModel = viewModel()) {
    var page by rememberSaveable { mutableStateOf(0) }
    var screen by rememberSaveable { mutableStateOf<String?>(null) }
    var recipeId by rememberSaveable { mutableStateOf<Long?>(null) }
    var editingCookId by rememberSaveable { mutableStateOf<Long?>(null) }
    LaunchedEffect(screen, recipeId, vm.loaded, vm.busy) {
        if (
            (screen == "recipe" || screen == "cook") &&
                recipeId != null &&
                vm.loaded &&
                !vm.busy &&
                vm.historyRecipeId != recipeId
        )
            vm.loadCooks(recipeId!!)
    }
    var planServings by rememberSaveable { mutableStateOf<Double?>(null) }
    var search by rememberSaveable { mutableStateOf("") }
    var confirm by remember { mutableStateOf<Pair<String, () -> Unit>?>(null) }
    val recipe = vm.recipes.find { it.id == recipeId }
    val close = {
        screen = null
        vm.clearError()
    }
    val context = LocalContext.current
    val authService = remember { AuthorizationService(context) }
    DisposableEffect(authService) { onDispose { authService.dispose() } }
    val scope = rememberCoroutineScope()
    var signInUrl by rememberSaveable { mutableStateOf("") }
    var signInResource by rememberSaveable { mutableStateOf("") }
    var signingIn by remember { mutableStateOf(false) }
    val launcher =
        rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result
            ->
            val intent = result.data
            val response = intent?.let { AuthorizationResponse.fromIntent(it) }
            if (response == null) {
                vm.authenticationError("Pocket ID sign-in was cancelled or could not be completed.")
            } else {
                authService.performTokenRequest(
                    response.createTokenExchangeRequest(mapOf("resource" to signInResource))
                ) { tokens, error ->
                    if (tokens == null || error != null)
                        vm.authenticationError("Pocket ID could not complete the token exchange.")
                    else {
                        val state = AuthState(response, null)
                        state.update(tokens, error)
                        vm.completeSignIn(signInUrl, state, close)
                    }
                }
            }
        }
    fun beginSignIn(rawUrl: String) {
        scope.launch {
            signingIn = true
            vm.clearError()
            try {
                val url = rawUrl.trim().trimEnd('/')
                require(url.startsWith("https://")) {
                    "Use an HTTPS Recetaco address for Pocket ID sign-in."
                }
                val config = loginConfig(url)
                val provider =
                    config.oidc
                        ?: error(
                            "This server has not enabled Pocket ID. Use the development connection option instead."
                        )
                val request = authorizationRequest(provider)
                signInUrl = url
                signInResource = provider.audience
                launcher.launch(authService.getAuthorizationRequestIntent(request))
            } catch (e: kotlinx.coroutines.CancellationException) {
                throw e
            } catch (e: Exception) {
                vm.authenticationError(e.message ?: "Could not start Pocket ID sign-in.")
            } finally {
                signingIn = false
            }
        }
    }
    fun open(which: String, r: Recipe? = null, servings: Double? = null) {
        recipeId = r?.id
        planServings = servings
        vm.clearError()
        screen = which
        if (which == "recipe" && r != null && !vm.busy) vm.loadCooks(r.id)
    }
    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(Icons.Outlined.LocalFireDepartment, null, tint = orange)
                        Spacer(Modifier.width(7.dp))
                        Text("recetaco.", fontFamily = FontFamily.Serif, fontSize = 30.sp)
                    }
                },
                actions = {
                    IconButton(
                        onClick = { vm.refresh() },
                        enabled = !vm.busy && vm.serverUrl.isNotBlank(),
                    ) {
                        Icon(Icons.Outlined.Refresh, "Refresh")
                    }
                    IconButton(onClick = { open("settings") }, enabled = !vm.busy) {
                        Icon(Icons.Outlined.Settings, "Connection settings")
                    }
                },
            )
        },
        bottomBar = {
            NavigationBar(containerColor = Color(0xFFEEEEE4)) {
                listOf("Recipes", "Meal plan", "Shopping").forEachIndexed { i, title ->
                    NavigationBarItem(
                        selected = page == i,
                        onClick = { page = i },
                        icon = {
                            Icon(
                                when (i) {
                                    0 -> Icons.AutoMirrored.Outlined.MenuBook
                                    1 -> Icons.Outlined.CalendarMonth
                                    else -> Icons.Outlined.ShoppingBasket
                                },
                                title,
                            )
                        },
                        label = { Text(title) },
                    )
                }
            }
        },
        floatingActionButton = {
            if (vm.loaded)
                FloatingActionButton(
                    onClick = {
                        open(
                            when (page) {
                                0 -> "edit"
                                1 -> "wish"
                                else -> "list"
                            }
                        )
                    },
                    containerColor = orange,
                    contentColor = Color.White,
                ) {
                    Icon(
                        Icons.Outlined.Add,
                        when (page) {
                            0 -> "New recipe"
                            1 -> "Add to week"
                            else -> "New list"
                        },
                    )
                }
        },
    ) { padding ->
        Column(Modifier.fillMaxSize().padding(padding)) {
            if (vm.busy) LinearProgressIndicator(Modifier.fillMaxWidth())
            if (vm.error != null && screen == null) ErrorCard(vm.error!!)
            if (vm.serverUrl.isBlank()) {
                EmptyState(
                    "Your kitchen, your data.",
                    "Connect to your Recetaco server to bring your recipes, meal plan, and shopping lists together.",
                ) {
                    Button(onClick = { open("settings") }) { Text("Connect to server") }
                }
            } else if (!vm.loaded) {
                EmptyState(
                    "Opening your kitchen…",
                    "Check your connection settings if this takes a while.",
                ) {
                    OutlinedButton(onClick = { vm.refresh() }, enabled = !vm.busy) { Text("Retry") }
                }
            } else
                when (page) {
                    0 -> {
                        LazyColumn(
                            Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(20.dp, 12.dp, 20.dp, 100.dp),
                            verticalArrangement = Arrangement.spacedBy(14.dp),
                        ) {
                            item {
                                Heading(
                                    "Keep the good ones.",
                                    "Your favorite recipes, all in one happy place.",
                                )
                            }
                            item {
                                OutlinedTextField(
                                    search,
                                    { search = it },
                                    label = { Text("Find a favorite") },
                                    leadingIcon = { Icon(Icons.Outlined.Search, null) },
                                    modifier = Modifier.fillMaxWidth(),
                                    singleLine = true,
                                )
                            }
                            if (vm.recipes.isEmpty())
                                item {
                                    EmptyState(
                                        "Every kitchen starts with a favorite.",
                                        "Tap + to add your first recipe.",
                                    )
                                }
                            val filtered =
                                vm.recipes.filter { it.title.contains(search, ignoreCase = true) }
                            if (filtered.isEmpty() && vm.recipes.isNotEmpty())
                                item { Text("No recipes found.") }
                            items(filtered, key = { it.id }) { r ->
                                Card(
                                    onClick = { open("recipe", r) },
                                    modifier = Modifier.fillMaxWidth(),
                                    colors =
                                        CardDefaults.cardColors(containerColor = Color(0xFFE9EDDF)),
                                ) {
                                    Row(
                                        Modifier.padding(20.dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        RecipePhoto(vm, r, Modifier.size(64.dp))
                                        Spacer(Modifier.width(18.dp))
                                        Column(Modifier.weight(1f)) {
                                            Text(
                                                r.title,
                                                fontSize = 22.sp,
                                                fontFamily = FontFamily.Serif,
                                            )
                                            Spacer(Modifier.height(7.dp))
                                            Text(
                                                "Serves ${qty(r.servings)} · ${r.ingredients.size} ingredients",
                                                fontSize = 12.sp,
                                            )
                                            Text(
                                                "★ ${r.rating?.let {"$it/5"} ?: "Not rated"} · ${r.cook_count} ${if(r.cook_count==1L) "cook" else "cooks"}",
                                                fontSize = 11.sp,
                                                color = orange,
                                            )
                                        }
                                        Icon(Icons.Outlined.NorthEast, null, tint = orange)
                                    }
                                }
                            }
                        }
                    }
                    1 ->
                        LazyColumn(
                            Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(20.dp, 12.dp, 20.dp, 100.dp),
                            verticalArrangement = Arrangement.spacedBy(14.dp),
                        ) {
                            item {
                                Heading(
                                    "Something to look forward to.",
                                    "A little planning. More room for living.",
                                )
                            }
                            item {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    IconButton(
                                        onClick = {
                                            vm.changeWeek(
                                                LocalDate.parse(vm.week).minusWeeks(1).toString()
                                            )
                                        },
                                        enabled = !vm.busy,
                                    ) {
                                        Icon(Icons.Outlined.ChevronLeft, "Previous week")
                                    }
                                    Column(Modifier.weight(1f)) {
                                        Text("Week of ${vm.week}", fontSize = 16.sp)
                                        Text(
                                            "Through ${LocalDate.parse(vm.week).plusDays(6)}",
                                            fontSize = 11.sp,
                                        )
                                    }
                                    IconButton(
                                        onClick = {
                                            vm.changeWeek(
                                                LocalDate.parse(vm.week).plusWeeks(1).toString()
                                            )
                                        },
                                        enabled = !vm.busy,
                                    ) {
                                        Icon(Icons.Outlined.ChevronRight, "Next week")
                                    }
                                }
                                TextButton(
                                    onClick = { vm.changeWeek(LocalDate.now().toString()) },
                                    enabled = !vm.busy,
                                ) {
                                    Text("This week")
                                }
                            }
                            item {
                                Text(
                                    "This week’s wishlist",
                                    fontSize = 25.sp,
                                    fontFamily = FontFamily.Serif,
                                )
                                Text(
                                    "Cook these whenever you feel like it. No days or slots required.",
                                    fontSize = 12.sp,
                                )
                            }
                            if (vm.wishes.isEmpty())
                                item {
                                    EmptyState(
                                        "A few things you’d love to cook.",
                                        "Tap + to add something to this week.",
                                    )
                                }
                            items(vm.wishes, key = { "wish-${it.id}" }) { wish ->
                                Card(Modifier.fillMaxWidth()) {
                                    Row(
                                        Modifier.padding(12.dp),
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Checkbox(
                                            checked = wish.cooked,
                                            onCheckedChange = { vm.cook(wish) },
                                            enabled = !vm.busy,
                                        )
                                        Column(Modifier.weight(1f)) {
                                            Text(
                                                wish.title,
                                                fontSize = 20.sp,
                                                fontFamily = FontFamily.Serif,
                                                textDecoration =
                                                    if (wish.cooked) TextDecoration.LineThrough
                                                    else TextDecoration.None,
                                            )
                                            Text(
                                                "Serves ${qty(wish.servings)} · ${if(wish.cooked) "Cooked" else "Sometime this week"}",
                                                fontSize = 11.sp,
                                            )
                                        }
                                        IconButton(
                                            onClick = {
                                                vm.recipes
                                                    .find { it.id == wish.recipe_id }
                                                    ?.let { open("shop", it, wish.servings) }
                                            },
                                            enabled = !vm.busy,
                                        ) {
                                            Icon(
                                                Icons.Outlined.ShoppingBasket,
                                                "Shop ingredients for ${wish.title}",
                                            )
                                        }
                                        IconButton(
                                            onClick = { vm.deleteWish(wish) },
                                            enabled = !vm.busy,
                                        ) {
                                            Icon(
                                                Icons.Outlined.Close,
                                                "Remove ${wish.title} from wishlist",
                                            )
                                        }
                                    }
                                }
                            }
                            item {
                                Button(
                                    onClick = { open("week-shop") },
                                    enabled = !vm.busy && vm.wishes.any { !it.cooked },
                                    modifier = Modifier.fillMaxWidth(),
                                ) {
                                    Text("Shop this week")
                                }
                                Spacer(Modifier.height(20.dp))
                                HorizontalDivider()
                                TextButton(onClick = { open("plan") }, enabled = !vm.busy) {
                                    Text("Optional: plan a meal for an exact date")
                                }
                            }

                            items(vm.meals, key = { it.id }) { m ->
                                Card(Modifier.fillMaxWidth()) {
                                    Column(
                                        Modifier.padding(16.dp),
                                        verticalArrangement = Arrangement.spacedBy(8.dp),
                                    ) {
                                        Text(
                                            "${m.date} · ${m.slot}",
                                            color = orange,
                                            fontSize = 12.sp,
                                        )
                                        Text(
                                            m.title,
                                            fontSize = 23.sp,
                                            fontFamily = FontFamily.Serif,
                                        )
                                        Text("Serves ${qty(m.servings)}", fontSize = 12.sp)
                                        Row(verticalAlignment = Alignment.CenterVertically) {
                                            TextButton(
                                                onClick = {
                                                    vm.recipes
                                                        .find { it.id == m.recipe_id }
                                                        ?.let { open("shop", it, m.servings) }
                                                },
                                                enabled = !vm.busy,
                                            ) {
                                                Icon(
                                                    Icons.Outlined.ShoppingBasket,
                                                    null,
                                                    Modifier.size(17.dp),
                                                )
                                                Spacer(Modifier.width(6.dp))
                                                Text("Add ingredients")
                                            }
                                            Spacer(Modifier.weight(1f))
                                            IconButton(
                                                onClick = {
                                                    confirm =
                                                        "Remove this meal from your plan?" to
                                                            {
                                                                vm.deleteMeal(m)
                                                            }
                                                },
                                                enabled = !vm.busy,
                                            ) {
                                                Icon(
                                                    Icons.Outlined.Close,
                                                    "Remove ${m.title} from plan",
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    else ->
                        LazyColumn(
                            Modifier.fillMaxSize(),
                            contentPadding = PaddingValues(20.dp, 12.dp, 20.dp, 100.dp),
                            verticalArrangement = Arrangement.spacedBy(10.dp),
                        ) {
                            item {
                                Heading(
                                    "A little list. A lighter week.",
                                    "Just what you need. Nothing to inventory.",
                                )
                            }
                            if (vm.lists.isEmpty())
                                item {
                                    EmptyState(
                                        "Less remembering. More cooking.",
                                        "Tap + to create your first shopping list.",
                                    )
                                }
                            else {
                                item {
                                    Choice(
                                        "Shopping list",
                                        vm.detail?.name ?: "Choose a list",
                                        vm.lists.map { it.id to it.name },
                                        !vm.busy,
                                    ) {
                                        vm.selectList(it)
                                    }
                                }
                                vm.detail?.let { list ->
                                    item {
                                        Row(verticalAlignment = Alignment.CenterVertically) {
                                            Text(
                                                "${list.items.count { !it.checked }} left to pick up",
                                                Modifier.weight(1f),
                                                fontSize = 13.sp,
                                            )
                                            TextButton(
                                                onClick = { open("item") },
                                                enabled = !vm.busy,
                                            ) {
                                                Icon(Icons.Outlined.Add, null, Modifier.size(16.dp))
                                                Text("Add item")
                                            }
                                            IconButton(
                                                onClick = {
                                                    confirm =
                                                        "Delete ${list.name} and all its items?" to
                                                            {
                                                                vm.deleteList(list.id) {}
                                                            }
                                                },
                                                enabled = !vm.busy,
                                            ) {
                                                Icon(
                                                    Icons.Outlined.DeleteOutline,
                                                    "Delete shopping list",
                                                )
                                            }
                                        }
                                    }
                                    if (list.items.isEmpty())
                                        item {
                                            Text(
                                                "Your list is ready. Add an item or ingredients from your recipe book.",
                                                fontSize = 13.sp,
                                            )
                                        }
                                    items(list.items, key = { it.id }) { item ->
                                        Row(
                                            Modifier.fillMaxWidth()
                                                .background(Color(0xFFF0F2E8))
                                                .padding(5.dp),
                                            verticalAlignment = Alignment.CenterVertically,
                                        ) {
                                            Checkbox(
                                                checked = item.checked,
                                                onCheckedChange = { vm.check(item) },
                                                enabled = !vm.busy,
                                            )
                                            Text(
                                                item.name,
                                                Modifier.weight(1f),
                                                textDecoration =
                                                    if (item.checked) TextDecoration.LineThrough
                                                    else TextDecoration.None,
                                                fontSize = 14.sp,
                                            )
                                            Text(
                                                "${qty(item.quantity)} ${item.unit}",
                                                fontSize = 12.sp,
                                            )
                                            IconButton(
                                                onClick = { vm.deleteItem(item) },
                                                enabled = !vm.busy,
                                            ) {
                                                Icon(
                                                    Icons.Outlined.Close,
                                                    "Remove ${item.name}",
                                                    Modifier.size(17.dp),
                                                )
                                            }
                                        }
                                    }
                                }
                            }
                        }
                }
        }
    }
    if (screen != null)
        Sheet(
            title =
                when (screen) {
                    "settings" -> "Connection settings"
                    "edit" -> if (recipe == null) "A new favorite" else "Edit recipe"
                    "recipe" -> recipe?.title ?: "Recipe"
                    "cook" -> if (editingCookId == null) "Log a cook" else "Edit cook"
                    "wish" -> "Add to this week"
                    "week-shop" -> "Shop the weekly wishlist"
                    "plan" -> "Put it on the menu"
                    "shop" -> "Add ingredients"
                    "list" -> "Make a little list"
                    else -> "Add shopping item"
                },
            busy = vm.busy,
            close = close,
        ) {
            vm.error?.let { ErrorCard(it) }
            when (screen) {
                "settings" ->
                    ConnectionForm(
                        vm.serverUrl,
                        vm.token,
                        vm.busy || signingIn,
                        ::beginSignIn,
                        { vm.signOut() },
                        vm.signedInWithPocketId,
                    ) { url, token ->
                        vm.connect(url, token, close)
                    }
                "edit" ->
                    RecipeEditor(recipe, vm, vm.busy) { data, photo, remove ->
                        vm.save(data, photo, remove, { recipeId = it.id }, close)
                    }
                "recipe" ->
                    recipe?.let { r ->
                        if (r.photo_version > 0)
                            RecipePhoto(vm, r, Modifier.fillMaxWidth().height(200.dp))
                        Text("Serves ${qty(r.servings)}", color = orange)
                        Choice(
                            "Recipe rating",
                            r.rating?.let { "$it/5" } ?: "Not rated",
                            listOf<Int?>(null, 1, 2, 3, 4, 5).map {
                                it to (it?.let { n -> "$n/5" } ?: "Not rated")
                            },
                            !vm.busy,
                        ) {
                            vm.rateRecipe(r.id, it)
                        }
                        Text(
                            "${r.cook_count} ${if(r.cook_count==1L) "cook" else "cooks"} · Average cook rating: ${r.average_cook_rating?.let {qty(it)+"/5"} ?: "Not rated"}",
                            fontSize = 12.sp,
                        )
                        r.last_cooked?.let { Text("Last cooked: $it", fontSize = 12.sp) }
                        Text("What you’ll need", fontSize = 21.sp, fontFamily = FontFamily.Serif)
                        r.ingredients.forEach { i ->
                            Row(Modifier.fillMaxWidth().padding(vertical = 6.dp)) {
                                Text(i.name, Modifier.weight(1f))
                                Text("${qty(i.quantity)} ${i.unit}")
                            }
                            HorizontalDivider()
                        }
                        Text("Let’s cook", fontSize = 21.sp, fontFamily = FontFamily.Serif)
                        Text(
                            r.instructions.ifBlank { "No instructions added yet." },
                            lineHeight = 25.sp,
                        )
                        if (r.source_url.isNotBlank())
                            Text(r.source_url, color = orange, fontSize = 12.sp)
                        Button(onClick = { open("shop", r) }, modifier = Modifier.fillMaxWidth()) {
                            Text("Add ingredients to list")
                        }
                        OutlinedButton(
                            onClick = { open("wish", r) },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text("Add to this week’s wishlist")
                        }
                        OutlinedButton(
                            onClick = { open("plan", r) },
                            modifier = Modifier.fillMaxWidth(),
                        ) {
                            Text("Plan this meal")
                        }
                        HorizontalDivider()
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                "Cook history",
                                Modifier.weight(1f),
                                fontSize = 23.sp,
                                fontFamily = FontFamily.Serif,
                            )
                            TextButton(
                                onClick = {
                                    editingCookId = null
                                    screen = "cook"
                                },
                                enabled = !vm.busy,
                            ) {
                                Text("Log a cook")
                            }
                        }
                        if (vm.historyRecipeId != r.id || vm.busy)
                            Text("Loading cook history…", fontSize = 12.sp)
                        else {
                            if (vm.cooks.isEmpty())
                                Text(
                                    "No cooks logged yet. Keep a note of what worked for next time.",
                                    fontSize = 12.sp,
                                )
                            vm.cooks.forEach { c ->
                                CookEntry(
                                    c,
                                    !vm.busy,
                                    edit = {
                                        editingCookId = c.id
                                        screen = "cook"
                                    },
                                    remove = {
                                        confirm =
                                            "Delete the cook logged on ${c.cooked_on}?" to
                                                {
                                                    vm.deleteCook(c)
                                                }
                                    },
                                )
                            }
                            TextButton(onClick = { vm.loadCooks(r.id) }, enabled = !vm.busy) {
                                Text("Refresh cook history")
                            }
                        }
                        Row {
                            TextButton(onClick = { open("edit", r) }) { Text("Edit recipe") }
                            Spacer(Modifier.weight(1f))
                            TextButton(
                                onClick = {
                                    confirm =
                                        "Delete this recipe and its meal-plan entries? Shopping items will stay." to
                                            {
                                                vm.deleteRecipe(r, close)
                                            }
                                },
                                enabled = !vm.busy,
                            ) {
                                Text("Delete", color = orange)
                            }
                        }
                    }
                "cook" ->
                    recipe?.let { r ->
                        CookEditor(
                            vm.cooks.find { it.id == editingCookId },
                            editingCookId,
                            r.id,
                            vm.busy,
                        ) { data ->
                            vm.saveCook(r.id, data, { editingCookId = it.id }) { screen = "recipe" }
                        }
                    }
                "wish" ->
                    WishEditor(vm.recipes, recipe, vm.week, vm.busy) {
                        vm.wish(it) {
                            page = 1
                            close()
                        }
                    }
                "week-shop" ->
                    WeekShoppingEditor(vm.lists, vm.busy) { id, name ->
                        vm.shopWeek(id, name) {
                            page = 2
                            close()
                        }
                    }
                "plan" ->
                    PlanEditor(vm.recipes, recipe, vm.busy) {
                        vm.plan(it) {
                            page = 1
                            close()
                        }
                    }
                "shop" ->
                    recipe?.let { r ->
                        ShoppingEditor(vm.lists, r, planServings, vm.busy) { id, name, servings ->
                            vm.addRecipe(id, name, r, servings) {
                                page = 2
                                close()
                            }
                        }
                    }
                "list" -> NameEditor("List name", "This week", vm.busy) { vm.createList(it, close) }
                "item" -> ItemEditor(vm.busy) { vm.addItem(it, close) }
            }
        }
    confirm?.let { (message, action) ->
        AlertDialog(
            onDismissRequest = { confirm = null },
            title = { Text("Are you sure?") },
            text = { Text(message) },
            confirmButton = {
                TextButton(
                    onClick = {
                        action()
                        confirm = null
                    },
                    enabled = !vm.busy,
                ) {
                    Text("Delete")
                }
            },
            dismissButton = { TextButton(onClick = { confirm = null }) { Text("Keep it") } },
        )
    }
}

@Composable
private fun Heading(title: String, subtitle: String) {
    Column(verticalArrangement = Arrangement.spacedBy(9.dp)) {
        Text(title, fontSize = 34.sp, lineHeight = 39.sp, fontFamily = FontFamily.Serif)
        Text(subtitle, fontSize = 13.sp, color = Color(0xFF788078))
        Spacer(Modifier.height(8.dp))
    }
}

@Composable
private fun EmptyState(title: String, description: String, action: @Composable () -> Unit = {}) {
    Column(
        Modifier.fillMaxWidth().padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(Icons.Outlined.Spa, null, tint = green, modifier = Modifier.size(35.dp))
        Text(title, fontSize = 25.sp, fontFamily = FontFamily.Serif)
        Text(description, fontSize = 13.sp, lineHeight = 21.sp)
        action()
    }
}

@Composable
private fun ErrorCard(message: String) {
    Surface(
        color = Color(0xFFF9E8DE),
        shape = MaterialTheme.shapes.small,
        modifier = Modifier.fillMaxWidth().padding(vertical = 8.dp),
    ) {
        Text(message, Modifier.padding(14.dp), color = Color(0xFF9B442B), fontSize = 13.sp)
    }
}

@Composable
private fun Sheet(
    title: String,
    busy: Boolean,
    close: () -> Unit,
    content: @Composable ColumnScope.() -> Unit,
) {
    androidx.compose.ui.window.Dialog(
        onDismissRequest = { if (!busy) close() },
        properties = androidx.compose.ui.window.DialogProperties(usePlatformDefaultWidth = false),
    ) {
        Surface(
            shape = MaterialTheme.shapes.large,
            modifier = Modifier.fillMaxWidth().padding(14.dp).heightIn(max = 720.dp),
        ) {
            Column(
                Modifier.imePadding().verticalScroll(rememberScrollState()).padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(13.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        title,
                        Modifier.weight(1f),
                        fontSize = 27.sp,
                        fontFamily = FontFamily.Serif,
                    )
                    IconButton(onClick = close, enabled = !busy) {
                        Icon(Icons.Outlined.Close, "Close dialog")
                    }
                }
                if (busy) LinearProgressIndicator(Modifier.fillMaxWidth())
                content()
            }
        }
    }
}

@Composable
private fun Field(
    label: String,
    value: String,
    onChange: (String) -> Unit,
    numeric: Boolean = false,
    lines: Int = 1,
) {
    OutlinedTextField(
        value,
        onChange,
        label = { Text(label) },
        modifier = Modifier.fillMaxWidth(),
        singleLine = lines == 1,
        minLines = lines,
        keyboardOptions =
            KeyboardOptions(keyboardType = if (numeric) KeyboardType.Decimal else KeyboardType.Text),
    )
}

@Composable
private fun <T> Choice(
    label: String,
    selected: String,
    options: List<Pair<T, String>>,
    enabled: Boolean = true,
    choose: (T) -> Unit,
) {
    var expanded by remember { mutableStateOf(false) }
    Column {
        Text(label, fontSize = 12.sp)
        Box {
            OutlinedButton(
                onClick = { expanded = true },
                enabled = enabled,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(selected, Modifier.weight(1f))
                Icon(Icons.Outlined.ExpandMore, null)
            }
            DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                options.forEach { (value, title) ->
                    DropdownMenuItem(
                        text = { Text(title) },
                        onClick = {
                            choose(value)
                            expanded = false
                        },
                    )
                }
            }
        }
    }
}

private fun validNumber(value: String): Boolean =
    value.toDoubleOrNull()?.let { it.isFinite() && it > 0 && it <= 1_000_000 } ?: false

@Composable
private fun ConnectionForm(
    initialUrl: String,
    initialToken: String,
    busy: Boolean,
    signIn: (String) -> Unit,
    signOut: () -> Unit,
    signedIn: Boolean,
    save: (String, String) -> Unit,
) {
    var url by rememberSaveable { mutableStateOf(initialUrl) }
    var token by remember { mutableStateOf(initialToken) }
    var developerMode by rememberSaveable { mutableStateOf(false) }
    Text(
        "Enter your Recetaco server’s HTTPS address. Pocket ID opens in your system browser for sign-in and passkeys.",
        fontSize = 13.sp,
        lineHeight = 22.sp,
    )
    Field("Server URL", url, { url = it })
    Button(
        onClick = { signIn(url) },
        enabled = !busy && url.isNotBlank(),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Sign in with Pocket ID")
    }
    if (signedIn)
        OutlinedButton(onClick = signOut, enabled = !busy, modifier = Modifier.fillMaxWidth()) {
            Text("Sign out of this app")
        }
    Text(
        "Authorization tokens are encrypted with Android Keystore. Sign-out clears this app’s credentials; your Pocket ID browser session is separate.",
        fontSize = 12.sp,
    )
    TextButton(onClick = { developerMode = !developerMode }, enabled = !busy) {
        Text("Development connection options")
    }
    if (developerMode) {
        OutlinedTextField(
            token,
            { token = it },
            label = { Text("Development API token (optional)") },
            visualTransformation = PasswordVisualTransformation(),
            singleLine = true,
            modifier = Modifier.fillMaxWidth(),
        )
        Text(
            "For a server without OIDC. The emulator reaches its host at http://10.0.2.2:8080.",
            fontSize = 12.sp,
        )
        OutlinedButton(
            onClick = { save(url, token) },
            enabled = !busy && url.isNotBlank(),
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text("Connect to development server")
        }
    }
}

private data class DraftIngredient(
    val name: String = "",
    val quantity: String = "1",
    val unit: String = "",
)

@Composable
private fun RecipeEditor(
    recipe: Recipe?,
    vm: RecetacoViewModel,
    busy: Boolean,
    save: (Recipe, Uri?, Boolean) -> Unit,
) {
    var photo by rememberSaveable { mutableStateOf<Uri?>(null) }
    var removePhoto by rememberSaveable { mutableStateOf(false) }
    val picker =
        rememberLauncherForActivityResult(ActivityResultContracts.PickVisualMedia()) { uri ->
            if (uri != null) {
                photo = uri
                removePhoto = false
            }
        }

    var title by rememberSaveable { mutableStateOf(recipe?.title.orEmpty()) }
    var servings by rememberSaveable { mutableStateOf(qty(recipe?.servings ?: 2.0)) }
    var instructions by rememberSaveable { mutableStateOf(recipe?.instructions.orEmpty()) }
    var url by rememberSaveable { mutableStateOf(recipe?.source_url.orEmpty()) }
    val ingredients = remember {
        mutableStateListOf<DraftIngredient>().apply {
            addAll(
                recipe?.ingredients?.map { DraftIngredient(it.name, qty(it.quantity), it.unit) }
                    ?: listOf(DraftIngredient())
            )
        }
    }
    Field("Recipe name", title, { title = it })
    Field("Servings", servings, { servings = it }, true)
    Field("Source URL (optional)", url, { url = it })
    if (photo != null) PickedPhoto(photo!!, Modifier.fillMaxWidth().height(160.dp))
    else if (!removePhoto && recipe != null && recipe.photo_version > 0)
        RecipePhoto(vm, recipe, Modifier.fillMaxWidth().height(160.dp))
    OutlinedButton(
        onClick = {
            picker.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly))
        },
        enabled = !busy,
    ) {
        Icon(Icons.Outlined.PhotoCamera, null)
        Spacer(Modifier.width(8.dp))
        Text("Choose recipe photo")
    }
    if (photo != null || (!removePhoto && (recipe?.photo_version ?: 0) > 0))
        TextButton(
            onClick = {
                photo = null
                removePhoto = true
            },
            enabled = !busy,
        ) {
            Text("Remove photo")
        }
    Text("Photos are resized before uploading and stored with your recipe.", fontSize = 12.sp)
    Text("Ingredients", fontSize = 22.sp, fontFamily = FontFamily.Serif)
    ingredients.forEachIndexed { n, ingredient ->
        Column(
            Modifier.background(Color(0xFFF0F2E8)).padding(10.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Field(
                "Ingredient ${n+1}",
                ingredient.name,
                { ingredients[n] = ingredient.copy(name = it) },
            )
            Row(
                horizontalArrangement = Arrangement.spacedBy(7.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(Modifier.weight(1f)) {
                    Field(
                        "Quantity",
                        ingredient.quantity,
                        { ingredients[n] = ingredient.copy(quantity = it) },
                        true,
                    )
                }
                Box(Modifier.weight(1f)) {
                    Field("Unit", ingredient.unit, { ingredients[n] = ingredient.copy(unit = it) })
                }
                IconButton(onClick = { ingredients.removeAt(n) }, enabled = ingredients.size > 1) {
                    Icon(Icons.Outlined.Remove, "Remove ingredient ${n+1}")
                }
            }
        }
    }
    TextButton(onClick = { ingredients.add(DraftIngredient()) }, enabled = ingredients.size < 200) {
        Icon(Icons.Outlined.Add, null)
        Text("Add ingredient")
    }
    Field("Instructions", instructions, { instructions = it }, lines = 4)
    val valid =
        title.isNotBlank() &&
            title.length <= 200 &&
            validNumber(servings) &&
            ingredients.all {
                it.name.isNotBlank() &&
                    it.name.length <= 200 &&
                    validNumber(it.quantity) &&
                    it.unit.length <= 80
            } &&
            (url.isEmpty() || url.startsWith("https://") || url.startsWith("http://"))
    Button(
        onClick = {
            save(
                Recipe(
                    recipe?.id ?: 0,
                    title,
                    servings.toDouble(),
                    instructions,
                    url,
                    ingredients.map { Ingredient(it.name, it.quantity.toDouble(), it.unit) },
                ),
                photo,
                removePhoto,
            )
        },
        enabled = valid && !busy,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Save recipe")
    }
}

@Composable
private fun PlanEditor(
    recipes: List<Recipe>,
    initial: Recipe?,
    busy: Boolean,
    save: (Meal) -> Unit,
) {
    var id by rememberSaveable { mutableStateOf(initial?.id ?: recipes.firstOrNull()?.id) }
    var servings by rememberSaveable {
        mutableStateOf(qty(initial?.servings ?: recipes.firstOrNull()?.servings ?: 2.0))
    }
    var date by rememberSaveable { mutableStateOf(LocalDate.now().toString()) }
    var slot by rememberSaveable { mutableStateOf("dinner") }
    val context = androidx.compose.ui.platform.LocalContext.current
    if (recipes.isEmpty()) {
        Text("Add a recipe to your book first, then come back to plan a meal.")
        return
    }
    Choice(
        "Recipe",
        recipes.find { it.id == id }?.title ?: "Choose a recipe",
        recipes.map { it.id to it.title },
    ) { selected ->
        id = selected
        servings = qty(recipes.first { it.id == selected }.servings)
    }
    OutlinedButton(
        onClick = {
            val d = LocalDate.parse(date)
            android.app
                .DatePickerDialog(
                    context,
                    { _, y, m, day -> date = LocalDate.of(y, m + 1, day).toString() },
                    d.year,
                    d.monthValue - 1,
                    d.dayOfMonth,
                )
                .show()
        },
        modifier = Modifier.fillMaxWidth(),
    ) {
        Icon(Icons.Outlined.CalendarMonth, null)
        Spacer(Modifier.width(8.dp))
        Text(date)
    }
    Choice("Meal", slot, listOf("breakfast", "lunch", "dinner", "snack").map { it to it }) {
        slot = it
    }
    Field("Servings", servings, { servings = it }, true)
    Button(
        onClick = {
            save(Meal(recipe_id = id!!, date = date, servings = servings.toDouble(), slot = slot))
        },
        enabled = id != null && validNumber(servings) && !busy,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Plan meal")
    }
}

@Composable
private fun ShoppingEditor(
    lists: List<ShoppingList>,
    recipe: Recipe,
    initialServings: Double?,
    busy: Boolean,
    save: (Long?, String, Double) -> Unit,
) {
    var id by rememberSaveable { mutableStateOf(lists.firstOrNull()?.id ?: 0L) }
    var name by rememberSaveable { mutableStateOf("This week") }
    var servings by rememberSaveable { mutableStateOf(qty(initialServings ?: recipe.servings)) }
    Text("From ${recipe.title}. Ingredients scale to your chosen servings.", fontSize = 13.sp)
    Choice(
        "Shopping list",
        lists.find { it.id == id }?.name ?: "Create a new list",
        lists.map { it.id to it.name } + (0L to "Create a new list"),
    ) {
        id = it
    }
    if (id == 0L) Field("New list name", name, { name = it })
    Field("Servings", servings, { servings = it }, true)
    Text(
        "Matching unchecked names and units combine. Each addition adds quantities again.",
        fontSize = 12.sp,
    )
    Button(
        onClick = { save(id.takeIf { it > 0 }, name, servings.toDouble()) },
        enabled = !busy && validNumber(servings) && (id != 0L || name.isNotBlank()),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Add ingredients")
    }
}

@Composable
private fun NameEditor(label: String, initial: String, busy: Boolean, save: (String) -> Unit) {
    var name by rememberSaveable { mutableStateOf(initial) }
    Field(label, name, { name = it })
    Button(
        onClick = { save(name) },
        enabled = !busy && name.isNotBlank() && name.length <= 200,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Create list")
    }
}

@Composable
private fun ItemEditor(busy: Boolean, save: (Ingredient) -> Unit) {
    var name by rememberSaveable { mutableStateOf("") }
    var quantity by rememberSaveable { mutableStateOf("1") }
    var unit by rememberSaveable { mutableStateOf("") }
    Field("Item name", name, { name = it })
    Field("Quantity", quantity, { quantity = it }, true)
    Field("Unit (optional)", unit, { unit = it })
    Button(
        onClick = { save(Ingredient(name, quantity.toDouble(), unit)) },
        enabled =
            !busy &&
                name.isNotBlank() &&
                name.length <= 200 &&
                unit.length <= 80 &&
                validNumber(quantity),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Add item")
    }
}

@Composable
private fun WishEditor(
    recipes: List<Recipe>,
    initial: Recipe?,
    week: String,
    busy: Boolean,
    save: (Wish) -> Unit,
) {
    var id by rememberSaveable { mutableStateOf(initial?.id ?: recipes.firstOrNull()?.id) }
    var servings by rememberSaveable {
        mutableStateOf(qty(initial?.servings ?: recipes.firstOrNull()?.servings ?: 2.0))
    }
    if (recipes.isEmpty()) {
        Text("Add a recipe to your book first.")
        return
    }
    Text("Week of $week. Decide when to cook it later.", fontSize = 13.sp)
    Choice(
        "Recipe",
        recipes.find { it.id == id }?.title ?: "Choose a recipe",
        recipes.map { it.id to it.title },
    ) { selected ->
        id = selected
        servings = qty(recipes.first { it.id == selected }.servings)
    }
    Field("Servings", servings, { servings = it }, true)
    Button(
        onClick = { save(Wish(recipe_id = id!!, week = week, servings = servings.toDouble())) },
        enabled = !busy && id != null && validNumber(servings),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Add to wishlist")
    }
}

@Composable
private fun WeekShoppingEditor(
    lists: List<ShoppingList>,
    busy: Boolean,
    save: (Long?, String) -> Unit,
) {
    var id by rememberSaveable { mutableStateOf(lists.firstOrNull()?.id ?: 0L) }
    var name by rememberSaveable { mutableStateOf("This week") }
    Text(
        "Add scaled ingredients for all uncooked wishlist meals. Cooked meals are skipped. Each addition adds quantities again.",
        fontSize = 13.sp,
        lineHeight = 22.sp,
    )
    Choice(
        "Shopping list",
        lists.find { it.id == id }?.name ?: "Create a new list",
        lists.map { it.id to it.name } + (0L to "Create a new list"),
    ) {
        id = it
    }
    if (id == 0L) Field("New list name", name, { name = it })
    Button(
        onClick = { save(id.takeIf { it > 0 }, name) },
        enabled = !busy && (id > 0 || name.isNotBlank()),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Add week’s ingredients")
    }
}
