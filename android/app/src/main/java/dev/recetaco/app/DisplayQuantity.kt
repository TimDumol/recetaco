package dev.recetaco.app

import java.util.Locale
import kotlin.math.abs
import kotlin.math.floor
import kotlin.math.round

/** Display only: editing, scaling and API payloads keep the original precision. */
fun displayQuantity(quantity: Double, unit: String): String {
    val metric = unit.trim().lowercase(Locale.ROOT) in setOf("g", "kg", "ml", "l", "mg")
    if (!metric) {
        val whole = floor(quantity).toLong()
        val fraction = quantity - whole
        val fractions = listOf(0.5 to "½", (1.0 / 3) to "⅓", (2.0 / 3) to "⅔",
            0.25 to "¼", 0.75 to "¾", 0.125 to "⅛", 0.375 to "⅜", 0.625 to "⅝", 0.875 to "⅞")
        fractions.firstOrNull { abs(fraction - it.first) < 0.00001 }?.let {
            return (if (whole > 0) whole.toString() + " " else "") + it.second
        }
    }
    val precision = if (unit.trim().lowercase(Locale.ROOT) in setOf("g", "ml", "mg") && quantity >= 10) 1.0 else 100.0
    val rounded = round(quantity * precision) / precision
    // Avoid rounding a small positive measurement down to zero.
    if (rounded == 0.0 && quantity > 0) return qty(quantity)
    return (if (abs(rounded - quantity) > 0.00001) "≈ " else "") + qty(rounded)
}
