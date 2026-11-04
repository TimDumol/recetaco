package dev.recetaco.app

import org.junit.Assert.assertEquals
import org.junit.Test

class DisplayQuantityTest {
    @Test fun recurringSpoonMeasuresUseFractions() {
        assertEquals("1 ⅓", displayQuantity(4.0 / 3, "cucharadita"))
        assertEquals("⅔", displayQuantity(2.0 / 3, "cucharadita"))
        assertEquals("5 ⅓", displayQuantity(16.0 / 3, "dientes"))
    }
    @Test fun metricRoundingIsExplicit() {
        assertEquals("≈ 267", displayQuantity(800.0 / 3, "g"))
        assertEquals("1200", displayQuantity(1200.0, "ml"))
        assertEquals("0.25", displayQuantity(0.25, "kg"))
    }
    @Test fun smallQuantitiesRemainVisible() {
        assertEquals("0.001", displayQuantity(0.001, "g"))
        assertEquals("½", displayQuantity(0.5, ""))
    }
}
