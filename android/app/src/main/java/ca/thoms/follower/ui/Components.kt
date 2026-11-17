// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)

package ca.thoms.follower.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.AlertDialog
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.ArrowDropDown
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.InputChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.foundation.layout.RowScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import ca.thoms.follower.data.STATUSES
import ca.thoms.follower.data.statusLabel
import coil3.compose.AsyncImage


@Composable
fun ScreenScaffold(
    title: String,
    onBack: (() -> Unit)? = null,
    bottomInset: Boolean = false,
    actions: @Composable RowScope.() -> Unit = {},
    content: @Composable (PaddingValues) -> Unit,
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    if (onBack != null) IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                actions = actions,
            )
        },
        contentWindowInsets = if (bottomInset) WindowInsets.navigationBars else WindowInsets(0, 0, 0, 0),
        content = content,
    )
}

@Composable
fun Avatar(url: String?, name: String, size: Dp = 44.dp, modifier: Modifier = Modifier) {
    if (url != null) {
        AsyncImage(
            model = url,
            contentDescription = name,
            contentScale = ContentScale.Crop,
            modifier = modifier.size(size).clip(CircleShape),
        )
    } else {
        Surface(
            shape = CircleShape,
            color = MaterialTheme.colorScheme.secondaryContainer,
            modifier = modifier.size(size),
        ) {
            Box(contentAlignment = Alignment.Center) {
                Text(name.take(1).uppercase(), style = MaterialTheme.typography.titleMedium)
            }
        }
    }
}

@Composable
fun SectionTitle(text: String, modifier: Modifier = Modifier) {
    Text(
        text, style = MaterialTheme.typography.titleSmall, color = MaterialTheme.colorScheme.primary,
        modifier = modifier.padding(top = 16.dp, bottom = 4.dp),
    )
}

@Composable
fun PickerButton(
    label: String,
    options: List<Pair<String?, String>>,
    onPick: (String?) -> Unit,
    modifier: Modifier = Modifier,
) {
    var open by remember { mutableStateOf(false) }
    Box(modifier) {
        OutlinedButton(onClick = { open = true }) {
            Text(label, maxLines = 1, overflow = TextOverflow.Ellipsis)
            Icon(Icons.Default.ArrowDropDown, contentDescription = null)
        }
        DropdownMenu(expanded = open, onDismissRequest = { open = false }) {
            options.forEach { (key, text) ->
                DropdownMenuItem(text = { Text(text) }, onClick = { open = false; onPick(key) })
            }
        }
    }
}

@Composable
fun RelationPicker(current: String?, onPick: (String) -> Unit, modifier: Modifier = Modifier, placeholder: String = "Relation") {
    PickerButton(
        label = current?.let(::statusLabel) ?: placeholder,
        options = STATUSES.map { it.key to it.value },
        onPick = { it?.let(onPick) },
        modifier = modifier,
    )
}

@Composable
fun TagChips(tags: List<String>, onRemove: ((String) -> Unit)? = null, onClick: ((String) -> Unit)? = null) {
    FlowRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        tags.forEach { t ->
            if (onRemove != null) {
                InputChip(selected = false, onClick = { onRemove(t) }, label = { Text(t) }, trailingIcon = { Icon(Icons.Default.Close, contentDescription = "Remove", modifier = Modifier.size(16.dp)) })
            } else {
                AssistChip(onClick = { onClick?.invoke(t) }, label = { Text(t) })
            }
        }
    }
}

@Composable
fun ChangesBlock(changes: Map<String, Pair<String, String>>) {
    if (changes.isEmpty()) return
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        SectionTitle("Changed since last check")
        changes.forEach { (k, v) ->
            Text("$k: ${v.first.shorten()} → ${v.second.shorten()}", style = MaterialTheme.typography.bodySmall)
        }
    }
}

private fun String.shorten(max: Int = 60) = if (length > max) take(max) + "…" else this

@Composable
fun ConfirmDialog(title: String, text: String, confirm: String, onConfirm: () -> Unit, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = { Text(text) },
        confirmButton = { TextButton(onClick = { onDismiss(); onConfirm() }) { Text(confirm) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

fun LazyListScope.section(content: @Composable ColumnScope.() -> Unit) {
    item { Column(content = content) }
}
