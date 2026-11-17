// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)

package ca.thoms.follower.ui

import android.graphics.BitmapFactory
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Image
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.ImageBitmap
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalUriHandler
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import ca.thoms.follower.AppViewModel
import ca.thoms.follower.Route
import ca.thoms.follower.data.ArchiveMsg
import ca.thoms.follower.data.Attachment
import ca.thoms.follower.data.humanSize
import ca.thoms.follower.data.pretty
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun ArchiveScreen(vm: AppViewModel) {
    var text by rememberSaveable { mutableStateOf("") }
    var author by rememberSaveable { mutableStateOf("") }
    val zone = vm.config.zone

    LaunchedEffect(Unit) { if (vm.config.ready) vm.loadAuthors() }

    ScreenScaffold(title = "Archive") { pad ->
        LazyColumn(Modifier.padding(pad), contentPadding = PaddingValues(16.dp)) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        text, { text = it }, label = { Text("Text contains") }, singleLine = true,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            author, { author = it }, label = { Text("Author (name or ID)") }, singleLine = true,
                            modifier = Modifier.weight(1f),
                        )
                        PickerButton(
                            label = "Pick",
                            options = listOf<Pair<String?, String>>(null to "Anyone") +
                                vm.authors.map { it.id.toString() to "${it.name} (${it.count})" },
                            onPick = { author = it ?: "" },
                        )
                    }
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Button(onClick = { vm.searchArchive(text, author) }, enabled = !vm.archiveBusy) {
                            Text(if (vm.archiveBusy) "Searching…" else "Search")
                        }
                        OutlinedButton(onClick = { vm.randomArchive(author) }, enabled = !vm.archiveBusy) { Text("Random") }
                    }
                }
            }
            val results = vm.archiveResults
            if (results != null) {
                item {
                    Text(
                        if (results.isEmpty()) "Nothing found." else "${results.size} result(s)",
                        style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(vertical = 12.dp),
                    )
                }
                items(results, key = { it.messageId }) { m ->
                    Card(Modifier.fillMaxWidth().padding(bottom = 10.dp).clickable { vm.push(Route.ArchiveRoute(m)) }) {
                        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                                Avatar(m.authorAvatar, m.authorName, size = 28.dp)
                                Text(m.authorName, style = MaterialTheme.typography.titleSmall, modifier = Modifier.weight(1f))
                                Text(m.messageTs.pretty(zone), style = MaterialTheme.typography.bodySmall)
                            }
                            Text(
                                m.content?.ifBlank { null } ?: "(no text)",
                                maxLines = 4, overflow = TextOverflow.Ellipsis, style = MaterialTheme.typography.bodyMedium,
                            )
                            if (m.attachments.isNotEmpty()) {
                                Text("${m.attachments.size} attachment(s)", style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun ArchiveDetailScreen(vm: AppViewModel, msg: ArchiveMsg) {
    val zone = vm.config.zone
    val uriHandler = LocalUriHandler.current
    var confirmDelete by remember { mutableStateOf(false) }

    ScreenScaffold(title = msg.authorName, onBack = { vm.pop() }, bottomInset = true) { pad ->
        LazyColumn(Modifier.padding(pad), contentPadding = PaddingValues(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            section {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    Avatar(msg.authorAvatar, msg.authorName)
                    Column {
                        Text(msg.authorName, style = MaterialTheme.typography.titleMedium)
                        Text("Sent ${msg.messageTs.pretty(zone)}", style = MaterialTheme.typography.bodySmall)
                        Text("Saved ${msg.savedAt.pretty(zone)}", style = MaterialTheme.typography.bodySmall)
                    }
                }
            }
            section {
                SelectionContainer {
                    Text(msg.content?.ifBlank { null } ?: "(no text)", style = MaterialTheme.typography.bodyLarge)
                }
            }
            if (msg.attachments.isNotEmpty()) {
                item { SectionTitle("Attachments") }
                items(msg.attachments, key = { it.id }) { a -> AttachmentCard(vm, a) }
            }
            section {
                Row(Modifier.padding(top = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    msg.jumpUrl?.let { url ->
                        OutlinedButton(onClick = { runCatching { uriHandler.openUri(url) } }) { Text("Open in Discord") }
                    }
                    OutlinedButton(onClick = { confirmDelete = true }) {
                        Icon(Icons.Default.Delete, contentDescription = null, tint = MaterialTheme.colorScheme.error)
                        Text("Delete", color = MaterialTheme.colorScheme.error)
                    }
                }
                Text("ID ${msg.messageId}", style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(top = 8.dp))
            }
        }
        if (confirmDelete) {
            ConfirmDialog(
                "Delete from archive?", "This also deletes the stored attachments.", "Delete",
                onConfirm = { vm.deleteArchive(msg.messageId) }, onDismiss = { confirmDelete = false },
            )
        }
    }
}

private fun decodeBitmap(bytes: ByteArray): ImageBitmap? {
    val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
    BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
    var sample = 1
    while (bounds.outWidth / sample > 2048 || bounds.outHeight / sample > 2048) sample *= 2
    val opts = BitmapFactory.Options().apply { inSampleSize = sample }
    return BitmapFactory.decodeByteArray(bytes, 0, bytes.size, opts)?.asImageBitmap()
}

@Composable
private fun AttachmentCard(vm: AppViewModel, a: Attachment) {
    val scope = rememberCoroutineScope()
    val uriHandler = LocalUriHandler.current
    val isImage = a.contentType?.startsWith("image/") == true
    var preview by remember { mutableStateOf<ImageBitmap?>(null) }
    var loading by remember { mutableStateOf(false) }

    val saver = rememberLauncherForActivityResult(ActivityResultContracts.CreateDocument(a.contentType ?: "*/*")) { uri ->
        if (uri != null) vm.saveAttachment(uri, a.id)
    }

    LaunchedEffect(a.id) {
        if (a.stored && isImage && a.storedSize <= 4L * 1024 * 1024) {
            loading = true
            val bytes = vm.attachmentBytes(a.id)
            preview = bytes?.let { withContext(Dispatchers.Default) { decodeBitmap(it) } }
            loading = false
        }
    }

    Card(Modifier.fillMaxWidth()) {
        Column(Modifier.padding(12.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(a.filename, style = MaterialTheme.typography.titleSmall)
            val where = when {
                !a.stored -> "link only"
                a.compressed -> "stored (compressed to ${humanSize(a.storedSize)})"
                else -> "stored"
            }
            Text(
                "${humanSize(a.size)} · ${a.contentType ?: "unknown"} · $where",
                style = MaterialTheme.typography.bodySmall,
            )
            preview?.let {
                Image(bitmap = it, contentDescription = a.filename, contentScale = ContentScale.Fit, modifier = Modifier.fillMaxWidth())
            }
            if (loading) Text("Loading…", style = MaterialTheme.typography.bodySmall)
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                if (a.stored) {
                    if (isImage && preview == null && !loading) {
                        OutlinedButton(onClick = {
                            scope.launch {
                                loading = true
                                val bytes = vm.attachmentBytes(a.id)
                                preview = bytes?.let { withContext(Dispatchers.Default) { decodeBitmap(it) } }
                                loading = false
                            }
                        }) { Text("Preview") }
                    }
                    TextButton(onClick = { saver.launch(a.filename) }) { Text("Save to device") }
                } else if (a.url != null) {
                    TextButton(onClick = { runCatching { uriHandler.openUri(a.url) } }) { Text("Open original link") }
                }
            }
        }
    }
}
