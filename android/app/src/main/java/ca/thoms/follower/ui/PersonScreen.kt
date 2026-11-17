// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)

package ca.thoms.follower.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.AssistChip
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import ca.thoms.follower.AppViewModel
import ca.thoms.follower.data.Person
import ca.thoms.follower.data.birthdayInput
import ca.thoms.follower.data.fmtBirthday
import ca.thoms.follower.data.parseBirthday
import ca.thoms.follower.data.pretty
import ca.thoms.follower.data.prettyDate
import ca.thoms.follower.data.statusLabel
import coil3.compose.AsyncImage
import java.time.Instant

@Composable
fun PersonScreen(vm: AppViewModel, userId: Long) {
    val d = vm.detail?.takeIf { it.userId == userId }
    val zone = vm.config.zone
    var showEdit by remember { mutableStateOf(false) }
    var confirmRemove by remember { mutableStateOf(false) }
    var tagText by remember { mutableStateOf("") }

    ScreenScaffold(
        title = d?.profile?.displayName ?: userId.toString(),
        onBack = { vm.pop() },
        bottomInset = true,
        actions = {
            if (d?.refreshing == true) CircularProgressIndicator(Modifier.padding(end = 12.dp).size(24.dp), strokeWidth = 2.dp)
            else IconButton(onClick = { vm.refreshDetail() }) { Icon(Icons.Default.Refresh, contentDescription = "Refresh from Discord") }
        },
    ) { pad ->
        if (d == null) {
            CircularProgressIndicator(Modifier.padding(pad).padding(24.dp))
            return@ScreenScaffold
        }
        val p = d.profile
        val person = d.person
        LazyColumn(Modifier.padding(pad), contentPadding = PaddingValues(16.dp)) {
            section {
                Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    if (p?.banner != null) {
                        AsyncImage(
                            model = p.banner, contentDescription = null, contentScale = ContentScale.Crop,
                            modifier = Modifier.fillMaxWidth().height(110.dp),
                        )
                    }
                    Row(Modifier.padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Avatar(p?.avatar, p?.displayName ?: "?", size = 64.dp)
                        Column {
                            Text(p?.displayName ?: userId.toString(), style = MaterialTheme.typography.titleLarge)
                            if (p != null) Text("@${p.username}", style = MaterialTheme.typography.bodyMedium)
                            SelectionContainer { Text(userId.toString(), style = MaterialTheme.typography.bodySmall) }
                        }
                    }
                    if (d.gone) {
                        Text(
                            "Couldn't fetch this account from Discord (deleted or unavailable). Showing the last saved profile.",
                            color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall,
                            modifier = Modifier.padding(top = 8.dp),
                        )
                    }
                    if (p != null) {
                        val created = runCatching { Instant.parse(p.createdAt.replace("+00:00", "Z")).pretty(zone) }
                            .getOrDefault(p.createdAt)
                        InfoLine("Created", created)
                        InfoLine("Badges", p.badges.joinToString(", ").ifEmpty { "none" })
                        if (p.bot) InfoLine("Bot", "yes")
                    }
                    ChangesBlock(d.changes)
                }
            }

            section {
                SectionTitle("Relation")
                if (person == null) {
                    Text("Not on your list yet.", style = MaterialTheme.typography.bodyMedium)
                    Button(onClick = { vm.addToList(userId) }, modifier = Modifier.padding(top = 8.dp)) { Text("Add to list") }
                } else {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        RelationPicker(person.relation, { vm.setRelation(userId, it) })
                        Text("Tracked since ${person.firstSeen.prettyDate(zone)}", style = MaterialTheme.typography.bodySmall)
                    }
                }
            }

            if (person != null) {
                section {
                    SectionTitle("Tags")
                    if (person.tags.isNotEmpty()) TagChips(person.tags, onRemove = { vm.removeTag(userId, it) })
                    Row(Modifier.padding(top = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedTextField(
                            value = tagText, onValueChange = { tagText = it }, singleLine = true,
                            label = { Text("Add tag(s), comma-separated") }, modifier = Modifier.weight(1f),
                        )
                        Button(
                            onClick = { vm.addTag(userId, tagText); tagText = "" },
                            enabled = tagText.isNotBlank(),
                        ) {
                            Icon(Icons.Default.Add, contentDescription = null)
                            Text("Add")
                        }
                    }
                    val suggestions = vm.allTags.map { it.first }.filter { it !in person.tags }.take(12)
                    if (suggestions.isNotEmpty()) {
                        Text("Existing tags", style = MaterialTheme.typography.bodySmall, modifier = Modifier.padding(top = 8.dp, bottom = 4.dp))
                        TagChips(suggestions, onClick = { vm.addTag(userId, it) })
                    }
                }

                section {
                    SectionTitle("Details")
                    if (person.birthdayMonth != null && person.birthdayDay != null) {
                        InfoLine("Birthday", fmtBirthday(person.birthdayMonth, person.birthdayDay, person.birthdayYear, zone))
                    }
                    person.metContext?.let { InfoLine("Know them from", it) }
                    person.notes?.let { InfoLine("Notes", it) }
                    OutlinedButton(onClick = { showEdit = true }, modifier = Modifier.padding(top = 8.dp)) {
                        Icon(Icons.Default.Edit, contentDescription = null)
                        Text("Edit details")
                    }
                }
            }

            section {
                SectionTitle("History")
                val h = vm.history
                if (h == null) {
                    OutlinedButton(onClick = { vm.loadHistory(userId) }) { Text("Show history") }
                } else {
                    if (h.statuses.isNotEmpty()) {
                        Text("Relation log", style = MaterialTheme.typography.labelLarge)
                        h.statuses.forEach { Text("${statusLabel(it.status)} · ${it.at.pretty(zone)}", style = MaterialTheme.typography.bodySmall) }
                    }
                    Text("Profile changes", style = MaterialTheme.typography.labelLarge, modifier = Modifier.padding(top = 8.dp))
                    if (h.profileChanges.isEmpty()) Text("No snapshots yet.", style = MaterialTheme.typography.bodySmall)
                    h.profileChanges.forEach { c ->
                        Column(Modifier.padding(vertical = 4.dp)) {
                            Text(
                                c.at.pretty(zone) + if (c.isFirst) " · first snapshot" else "",
                                style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.primary,
                            )
                            c.changes.forEach { (k, v) ->
                                Text("$k: ${v.first.take(40)} → ${v.second.take(40)}", style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
            }

            if (person != null) {
                section {
                    OutlinedButton(
                        onClick = { confirmRemove = true },
                        modifier = Modifier.padding(top = 24.dp),
                    ) {
                        Icon(Icons.Default.Delete, contentDescription = null, tint = MaterialTheme.colorScheme.error)
                        Text("Remove from list", color = MaterialTheme.colorScheme.error)
                    }
                }
            }
        }

        if (showEdit && person != null) {
            EditDetailsDialog(person, onDismiss = { showEdit = false }) { notes, met, y, mo, day ->
                vm.saveDetails(userId, notes, met, y, mo, day)
            }
        }
        if (confirmRemove) {
            ConfirmDialog(
                "Remove from list?", "Tags and details will be deleted. Profile snapshots are kept.", "Remove",
                onConfirm = { vm.removePerson(userId) }, onDismiss = { confirmRemove = false },
            )
        }
    }
}

@Composable
private fun InfoLine(label: String, value: String) {
    Row(Modifier.padding(vertical = 2.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.primary, modifier = Modifier.width(96.dp))
        Text(value, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
    }
}

@Composable
private fun EditDetailsDialog(
    person: Person,
    onDismiss: () -> Unit,
    onSave: (notes: String, met: String, y: Int?, mo: Int?, d: Int?) -> Unit,
) {
    var notes by remember { mutableStateOf(person.notes ?: "") }
    var met by remember { mutableStateOf(person.metContext ?: "") }
    var bd by remember { mutableStateOf(birthdayInput(person.birthdayMonth, person.birthdayDay, person.birthdayYear)) }
    var error by remember { mutableStateOf<String?>(null) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Person details") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(notes, { notes = it.take(1000) }, label = { Text("Notes") }, minLines = 3, modifier = Modifier.fillMaxWidth())
                OutlinedTextField(met, { met = it.take(60) }, label = { Text("Where you know them from") }, singleLine = true, modifier = Modifier.fillMaxWidth())
                OutlinedTextField(
                    bd, { bd = it.take(10); error = null }, label = { Text("Birthday (YYYY-MM-DD or MM-DD)") },
                    singleLine = true, isError = error != null, supportingText = { error?.let { Text(it) } },
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        },
        confirmButton = {
            TextButton(onClick = {
                try {
                    val (y, mo, d) = parseBirthday(bd)
                    onSave(notes, met, y, mo, d)
                    onDismiss()
                } catch (e: IllegalArgumentException) {
                    error = "Use 1999-03-05 or 03-05"
                }
            }) { Text("Save") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
