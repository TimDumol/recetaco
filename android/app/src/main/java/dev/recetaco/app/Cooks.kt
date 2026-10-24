package dev.recetaco.app

import android.app.DatePickerDialog
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import java.time.LocalDate

@Composable
fun CookEntry(cook: Cook, enabled: Boolean, edit: () -> Unit, remove: () -> Unit) {
    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
            Text(
                "${cook.cooked_on}${if(cook.cook_name.isNotBlank()) " · ${cook.cook_name}" else ""}",
                fontSize = 14.sp,
            )
            Text("Cook rating: ${cook.rating?.let {"$it/5"} ?: "Not rated"}", fontSize = 12.sp)
            if (cook.went_well.isNotBlank()) {
                Text("Went well", fontSize = 12.sp)
                Text(cook.went_well, fontSize = 13.sp)
            }
            if (cook.to_improve.isNotBlank()) {
                Text("To improve", fontSize = 12.sp)
                Text(cook.to_improve, fontSize = 13.sp)
            }
            if (cook.notes.isNotBlank()) {
                Text("Notes", fontSize = 12.sp)
                Text(cook.notes, fontSize = 13.sp)
            }
            Row {
                TextButton(onClick = edit, enabled = enabled) { Text("Edit cook") }
                Spacer(Modifier.weight(1f))
                TextButton(onClick = remove, enabled = enabled) { Text("Delete cook") }
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CookEditor(cook: Cook?, id: Long?, recipeId: Long, busy: Boolean, save: (Cook) -> Unit) {
    var date by rememberSaveable { mutableStateOf(cook?.cooked_on ?: LocalDate.now().toString()) }
    var rating by rememberSaveable { mutableStateOf(cook?.rating ?: 0) }
    var cookName by rememberSaveable { mutableStateOf(cook?.cook_name.orEmpty()) }
    var wentWell by rememberSaveable { mutableStateOf(cook?.went_well.orEmpty()) }
    var improve by rememberSaveable { mutableStateOf(cook?.to_improve.orEmpty()) }
    var notes by rememberSaveable { mutableStateOf(cook?.notes.orEmpty()) }
    val context = LocalContext.current
    OutlinedButton(
        onClick = {
            val d = LocalDate.parse(date)
            DatePickerDialog(
                    context,
                    { _, y, m, day -> date = LocalDate.of(y, m + 1, day).toString() },
                    d.year,
                    d.monthValue - 1,
                    d.dayOfMonth,
                )
                .show()
        },
        enabled = !busy,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Cook date: $date")
    }
    var expanded by remember { mutableStateOf(false) }
    Text("Cook rating", fontSize = 12.sp)
    Box {
        OutlinedButton(
            onClick = { expanded = true },
            enabled = !busy,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Text(if (rating == 0) "Not rated" else "$rating/5")
        }
        DropdownMenu(expanded, onDismissRequest = { expanded = false }) {
            (0..5).forEach { n ->
                DropdownMenuItem(
                    text = { Text(if (n == 0) "Not rated" else "$n/5") },
                    onClick = {
                        rating = n
                        expanded = false
                    },
                )
            }
        }
    }
    OutlinedTextField(
        cookName,
        { cookName = it },
        label = { Text("Cooked by (optional)") },
        modifier = Modifier.fillMaxWidth(),
        singleLine = true,
    )
    OutlinedTextField(
        wentWell,
        { wentWell = it },
        label = { Text("Went well") },
        modifier = Modifier.fillMaxWidth(),
        minLines = 3,
    )
    OutlinedTextField(
        improve,
        { improve = it },
        label = { Text("To improve") },
        modifier = Modifier.fillMaxWidth(),
        minLines = 3,
    )
    OutlinedTextField(
        notes,
        { notes = it },
        label = { Text("Notes") },
        modifier = Modifier.fillMaxWidth(),
        minLines = 3,
    )
    Text(
        "This records one cooking occasion. Wishlist checkboxes and the shared recipe rating stay separate.",
        fontSize = 12.sp,
    )
    Button(
        onClick = {
            save(
                Cook(
                    id = id ?: 0,
                    recipe_id = recipeId,
                    cooked_on = date,
                    rating = rating.takeIf { it > 0 },
                    cook_name = cookName,
                    went_well = wentWell,
                    to_improve = improve,
                    notes = notes,
                )
            )
        },
        enabled =
            !busy &&
                cookName.length <= 100 &&
                wentWell.length <= 10000 &&
                improve.length <= 10000 &&
                notes.length <= 10000,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text("Save cook")
    }
}
