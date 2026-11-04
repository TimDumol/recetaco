package dev.recetaco.app

import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun CookingMethod(recipe: Recipe, visible: Boolean = true) {
    val view = LocalView.current
    DisposableEffect(view, visible) {
        val previous = view.keepScreenOn
        if (visible) view.keepScreenOn = true
        onDispose { if (visible) view.keepScreenOn = previous }
    }
    if (recipe.steps.isEmpty()) {
        if (!visible) return
        Text(recipe.instructions.ifBlank { "No instructions added yet." }, lineHeight = 25.sp)
        return
    }
    var flow by remember(recipe.id) { mutableStateOf(false) }
    var done by remember(recipe.id) { mutableStateOf(setOf<Int>()) }
    val expanded = remember(recipe.id) { mutableStateMapOf<Int, Boolean>() }
    if (!visible) return
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        FilterChip(selected = !flow, onClick = { flow = false }, label = { Text("Method") })
        FilterChip(selected = flow, onClick = { flow = true }, label = { Text("Flow") })
    }
    if (flow) Text("Independent steps are grouped together. Open a step for full directions.", fontSize = 13.sp)
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text("${done.size} of ${recipe.steps.size} complete", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.secondary)
        Text("Screen stays awake", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
    LinearProgressIndicator(progress = { done.size.toFloat() / recipe.steps.size }, modifier = Modifier.fillMaxWidth())
    val levels = mutableListOf<Int>()
    recipe.steps.forEach { step ->
        levels.add(if (step.after.isEmpty()) 0 else 1 + step.after.maxOf { levels.getOrElse(it) { 0 } })
    }
    val groups = if (flow) recipe.steps.indices.groupBy { levels[it] }.toSortedMap().values.toList()
        else listOf(recipe.steps.indices.toList())
    groups.forEachIndexed { stage, indices ->
        if (flow) {
            HorizontalDivider()
            Text(
                (if (stage == 0) "Start here" else "↓ Continue · stage ${stage + 1}") +
                    (if (indices.size > 1) " · ${indices.size} parallel branches" else ""),
                color = MaterialTheme.colorScheme.secondary,
            )
        }
        indices.forEach { index ->
            val step = recipe.steps[index]
            val content: @Composable () -> Unit = {
                if (flow) Text(
                    if (step.after.isEmpty()) "Can start independently"
                    else "After " + step.after.joinToString(" + ") { "step ${it + 1}" },
                    fontSize = 12.sp,
                )
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(checked = index in done, onCheckedChange = {
                        done = if (it) done + index else done - index
                    })
                    Text("Step ${index + 1}" + if (index in done) " · Done" else "")
                }
                if (flow) {
                    Text(step.text.substringBefore(". ").let { if (it.endsWith('.')) it else "$it." })
                    TextButton(onClick = { expanded[index] = expanded[index] != true }) {
                        Text(if (expanded[index] == true) "Hide full directions" else "Full directions for step ${index + 1}")
                    }
                }
                if (!flow || expanded[index] == true) Text(step.text, lineHeight = 25.sp)
                var showIngredients by remember(recipe.id, index) { mutableStateOf(step.ingredients.size <= 3) }
                if (step.ingredients.isNotEmpty()) {
                    TextButton(onClick = { showIngredients = !showIngredients }) {
                        Text("${step.ingredients.size} ingredients · ${if (showIngredients) "Hide" else "Show amounts"}")
                    }
                }
                if (showIngredients) step.ingredients.forEach { usage ->
                    recipe.ingredients.getOrNull(usage.ingredient)?.let { ingredient ->
                        IngredientLine(
                            ingredient.name,
                            usage.quantity ?: ingredient.quantity,
                            ingredient.unit,
                        )
                    }
                }
            }
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLowest),
            ) {
                Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) { content() }
            }
        }
    }
    if (recipe.instructions.isNotBlank()) {
        var showOriginal by remember(recipe.id) { mutableStateOf(false) }
        TextButton(onClick = { showOriginal = !showOriginal }) { Text("Full recipe, tips and alternatives") }
        if (showOriginal) Text(recipe.instructions, lineHeight = 25.sp)
    }
}
