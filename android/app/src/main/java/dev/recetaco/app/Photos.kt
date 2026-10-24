package dev.recetaco.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Matrix
import android.net.Uri
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Restaurant
import androidx.compose.material3.Icon
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import androidx.exifinterface.media.ExifInterface
import java.io.ByteArrayOutputStream
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

suspend fun preparePhoto(context: Context, uri: Uri): ByteArray =
    withContext(Dispatchers.IO) {
        val resolver = context.contentResolver
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        resolver.openInputStream(uri).use { BitmapFactory.decodeStream(it, null, bounds) }
        require(bounds.outWidth > 0 && bounds.outHeight > 0) { "This photo could not be opened." }
        var sample = 1
        while (bounds.outWidth / sample > 2400 || bounds.outHeight / sample > 2400) sample *= 2
        val bitmap =
            resolver.openInputStream(uri).use {
                BitmapFactory.decodeStream(
                    it,
                    null,
                    BitmapFactory.Options().apply { inSampleSize = sample },
                )
            } ?: error("Could not read photo.")
        try {
            val orientation =
                resolver.openInputStream(uri).use {
                    if (it != null)
                        ExifInterface(it)
                            .getAttributeInt(
                                ExifInterface.TAG_ORIENTATION,
                                ExifInterface.ORIENTATION_NORMAL,
                            )
                    else ExifInterface.ORIENTATION_NORMAL
                }
            val matrix = Matrix()
            when (orientation) {
                ExifInterface.ORIENTATION_ROTATE_90 -> matrix.postRotate(90f)
                ExifInterface.ORIENTATION_ROTATE_180 -> matrix.postRotate(180f)
                ExifInterface.ORIENTATION_ROTATE_270 -> matrix.postRotate(270f)
                ExifInterface.ORIENTATION_FLIP_HORIZONTAL -> matrix.postScale(-1f, 1f)
                ExifInterface.ORIENTATION_FLIP_VERTICAL -> matrix.postScale(1f, -1f)
                ExifInterface.ORIENTATION_TRANSPOSE -> {
                    matrix.postRotate(90f)
                    matrix.postScale(-1f, 1f)
                }
                ExifInterface.ORIENTATION_TRANSVERSE -> {
                    matrix.postRotate(270f)
                    matrix.postScale(-1f, 1f)
                }
            }
            val ratio = minOf(1f, 1200f / bitmap.width, 1200f / bitmap.height)
            matrix.postScale(ratio, ratio)
            val resized =
                Bitmap.createBitmap(bitmap, 0, 0, bitmap.width, bitmap.height, matrix, true)
            try {
                ByteArrayOutputStream().use { out ->
                    check(resized.compress(Bitmap.CompressFormat.JPEG, 82, out))
                    out.toByteArray()
                }
            } finally {
                if (resized !== bitmap) resized.recycle()
            }
        } finally {
            bitmap.recycle()
        }
    }

@Composable
fun RecipePhoto(vm: RecetacoViewModel, recipe: Recipe, modifier: Modifier = Modifier) {
    val bitmap by
        produceState<ImageBitmap?>(null, recipe.id, recipe.photo_version, vm.serverUrl) {
            value = null
            if (recipe.photo_version > 0)
                value =
                    runCatching {
                            val bytes = vm.photo(recipe.id)
                            withContext(Dispatchers.Default) {
                                BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.asImageBitmap()
                            }
                        }
                        .getOrNull()
        }
    if (bitmap != null) Image(bitmap!!, recipe.title, modifier, contentScale = ContentScale.Crop)
    else
        Icon(
            Icons.Outlined.Restaurant,
            if (recipe.photo_version > 0) "Photo unavailable" else null,
            modifier.size(34.dp),
        )
}

@Composable
fun PickedPhoto(uri: Uri, modifier: Modifier = Modifier) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val bitmap by
        produceState<ImageBitmap?>(null, uri) {
            value =
                runCatching {
                        val bytes = preparePhoto(context, uri)
                        BitmapFactory.decodeByteArray(bytes, 0, bytes.size)?.asImageBitmap()
                    }
                    .getOrNull()
        }
    if (bitmap != null)
        Image(bitmap!!, "Selected recipe photo", modifier, contentScale = ContentScale.Crop)
}
