package dev.recetaco.app

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

/** A shared visual vocabulary for recipe, step, and shopping quantities. */
@Composable
fun AmountBadge(quantity: Double, unit: String, muted: Boolean = false) {
    Surface(
        color = if (muted) MaterialTheme.colorScheme.surfaceVariant else MaterialTheme.colorScheme.secondaryContainer,
        contentColor = if (muted) MaterialTheme.colorScheme.onSurfaceVariant else MaterialTheme.colorScheme.onSecondaryContainer,
        shape = RoundedCornerShape(10.dp),
        modifier = Modifier.widthIn(max = 112.dp),
    ) {
        Text(
            buildAnnotatedString {
                withStyle(SpanStyle(fontWeight = FontWeight.SemiBold, fontSize = 16.sp)) {
                    append(displayQuantity(quantity, unit))
                }
                if (unit.isNotBlank()) withStyle(SpanStyle(fontWeight = FontWeight.Normal, fontSize = 13.sp)) {
                    append(" " + unit.trim())
                }
            },
            Modifier.padding(horizontal = 10.dp, vertical = 6.dp),
            fontSize = 14.sp,
            fontWeight = FontWeight.SemiBold,
        )
    }
}

@Composable
fun IngredientLine(name: String, quantity: Double, unit: String) {
    Row(
        Modifier.fillMaxWidth().padding(vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Text(
            name,
            Modifier.weight(1f),
            fontSize = 16.sp,
            lineHeight = 23.sp,
            fontWeight = FontWeight.Medium,
        )
        // Bound unusually long units so the ingredient name always retains room.
        Box(Modifier.widthIn(max = 112.dp)) { AmountBadge(quantity, unit) }
    }
}
