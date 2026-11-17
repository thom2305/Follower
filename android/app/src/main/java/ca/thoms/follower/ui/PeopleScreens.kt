// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

@file:OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)

package ca.thoms.follower.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Button
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Card
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import ca.thoms.follower.AppViewModel
import ca.thoms.follower.data.LookupEntry
import ca.thoms.follower.Route
import ca.thoms.follower.data.PersonEntry
import ca.thoms.follower.data.STATUSES
import ca.thoms.follower.data.fmtBirthday
import ca.thoms.follower.data.nextBirthday
import ca.thoms.follower.data.parseIds
import ca.thoms.follower.data.statusLabel
import java.time.LocalDate
import java.time.temporal.ChronoUnit

@Composable
fun PeopleScreen(vm: AppViewModel) {
    var query by rememberSaveable { mutableStateOf("") }
    var relation by rememberSaveable { mutableStateOf<String?>(null) }
    var tag by rememberSaveable { mutableStateOf<String?>(null) }
    val all = vm.people

    val tags = remember(all) {
        all.flatMap { it.person.tags }.groupingBy { it }.eachCount().entries
            .sortedWith(compareByDescending<Map.Entry<String, Int>> { it.value }.thenBy { it.key }).map { it.key }
    }
    val filtered = remember(all, query, relation, tag) {
        val q = query.trim().lowercase()
        all.filter { e ->
            (relation == null || e.person.relation == relation) &&
                (tag == null || tag in e.person.tags) &&
                (q.isEmpty() ||
                    e.name.lowercase().contains(q) ||
                    (e.profile?.username?.lowercase()?.contains(q) == true) ||
                    e.person.userId.toString().contains(q) ||
                    (e.person.notes?.lowercase()?.contains(q) == true) ||
                    (e.person.metContext?.lowercase()?.contains(q) == true))
        }
    }

    ScreenScaffold(
        title = "People (${filtered.size})",
        actions = {
            TextButton(
                onClick = { vm.refreshAll(filtered.map { it.person.userId }) },
                enabled = vm.progress == null && filtered.isNotEmpty(),
            ) { Text("Update all") }
            IconButton(onClick = { vm.loadPeople() }) { Icon(Icons.Default.Refresh, contentDescription = "Reload list") }
        },
    ) { pad ->
        Column(Modifier.padding(pad)) {
            OutlinedTextField(
                value = query, onValueChange = { query = it }, singleLine = true,
                label = { Text("Search name, @username, ID, notes") },
                modifier = Modifier.fillMaxWidth().padding(horizontal = 16.dp),
            )
            LazyRow(
                contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 16.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                item { FilterChip(selected = relation == null, onClick = { relation = null }, label = { Text("All") }) }
                items(STATUSES.entries.toList()) { (k, v) ->
                    FilterChip(selected = relation == k, onClick = { relation = if (relation == k) null else k }, label = { Text(v) })
                }
            }
            if (tags.isNotEmpty()) {
                LazyRow(
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 16.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    items(tags) { t ->
                        FilterChip(selected = tag == t, onClick = { tag = if (tag == t) null else t }, label = { Text("#$t") })
                    }
                }
            }
            vm.progress?.let { Text(it, modifier = Modifier.padding(16.dp, 8.dp), style = MaterialTheme.typography.bodySmall) }
            if (vm.peopleLoading && all.isEmpty()) {
                CircularProgressIndicator(Modifier.padding(24.dp))
            }
            if (!vm.peopleLoading && filtered.isEmpty()) {
                Text(
                    if (all.isEmpty()) "Nobody on your list yet. Use the Lookup tab to add people." else "Nobody matches that.",
                    modifier = Modifier.padding(24.dp),
                )
            }
            LazyColumn {
                items(filtered, key = { it.person.userId }) { e ->
                    Column {
                        PersonRow(e) { vm.push(Route.PersonRoute(e.person.userId)) }
                        HorizontalDivider()
                    }
                }
            }
        }
    }
}

@Composable
fun PersonRow(e: PersonEntry, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().clickable(onClick = onClick).padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Avatar(e.profile?.avatar, e.name)
        Column(Modifier.weight(1f)) {
            Text(e.name, style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis)
            val sub = buildString {
                e.profile?.let { append("@${it.username} · ") }
                append(statusLabel(e.person.relation))
            }
            Text(sub, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
            if (e.person.tags.isNotEmpty()) {
                Text(
                    e.person.tags.joinToString(" ") { "#$it" }, style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.primary, maxLines = 1, overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@Composable
fun LookupScreen(vm: AppViewModel) {
    var text by rememberSaveable { mutableStateOf("") }
    var save by rememberSaveable { mutableStateOf(false) }
    var relation by rememberSaveable { mutableStateOf<String?>(null) }
    var tag by rememberSaveable { mutableStateOf("") }
    val ids = remember(text) { parseIds(text) }
    val results = vm.lookupResults
    val untracked = results.filter { it.person == null }.map { it.userId }

    ScreenScaffold(title = "Lookup") { pad ->
        LazyColumn(Modifier.padding(pad), contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp)) {
            item {
                Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = text, onValueChange = { text = it },
                        label = { Text("User IDs or <@mentions>") },
                        supportingText = { Text("${ids.size} ID(s) found · paste as many as you like (max 25)") },
                        modifier = Modifier.fillMaxWidth(), minLines = 2,
                    )
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        Switch(checked = save, onCheckedChange = { save = it })
                        Text("Also add to my list")
                    }
                    if (save) {
                        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            RelationPicker(relation, { relation = it })
                            OutlinedTextField(
                                value = tag, onValueChange = { tag = it }, singleLine = true,
                                label = { Text("Tag (optional)") }, modifier = Modifier.weight(1f),
                            )
                        }
                    }
                    Button(
                        onClick = { vm.lookup(ids, save, relation, tag.ifBlank { null }) },
                        enabled = ids.isNotEmpty() && !vm.lookupBusy,
                    ) { Text(if (vm.lookupBusy) "Looking up…" else "Look up") }
                    if (untracked.size > 1) {
                        OutlinedButton(onClick = { vm.saveLookupToList(untracked) }) { Text("Save all ${untracked.size} to list") }
                    }
                }
            }
            items(results, key = { it.userId }) { r ->
                LookupCard(r, vm)
            }
        }
    }
}

@Composable
private fun LookupCard(r: LookupEntry, vm: AppViewModel) {
    Card(Modifier.fillMaxWidth().padding(top = 12.dp).clickable { vm.push(Route.PersonRoute(r.userId)) }) {
        Column(Modifier.padding(12.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                Avatar(r.profile?.avatar, r.profile?.displayName ?: "?")
                Column(Modifier.weight(1f)) {
                    Text(r.profile?.displayName ?: "Not found", style = MaterialTheme.typography.titleMedium)
                    Text(
                        (r.profile?.let { "@${it.username} · " } ?: "") + r.userId,
                        style = MaterialTheme.typography.bodySmall,
                    )
                    Text(
                        r.person?.let { "In your list: ${statusLabel(it.relation)}" } ?: "Not tracked",
                        style = MaterialTheme.typography.bodySmall,
                    )
                }
                if (r.person == null) {
                    TextButton(onClick = { vm.saveLookupToList(listOf(r.userId)) }) { Text("Save") }
                }
            }
            ChangesBlock(r.changes)
        }
    }
}

@Composable
fun BirthdaysScreen(vm: AppViewModel) {
    var days by remember { mutableFloatStateOf(60f) }
    val zone = vm.config.zone
    val today = remember { LocalDate.now(zone) }
    val upcoming = remember(vm.people, days) {
        vm.people.filter { it.person.birthdayMonth != null && it.person.birthdayDay != null }
            .map { e ->
                val nxt = nextBirthday(e.person.birthdayMonth!!, e.person.birthdayDay!!, today)
                Triple(ChronoUnit.DAYS.between(today, nxt), e, nxt)
            }
            .filter { it.first <= days.toLong() }
            .sortedBy { it.first }
    }
    ScreenScaffold(title = "Birthdays") { pad ->
        Column(Modifier.padding(pad).padding(horizontal = 16.dp)) {
            Text("Next ${days.toInt()} days", style = MaterialTheme.typography.titleSmall)
            Slider(value = days, onValueChange = { days = it }, valueRange = 7f..366f)
            if (upcoming.isEmpty()) Text("No birthdays in that window.", modifier = Modifier.padding(top = 8.dp))
            LazyColumn {
                items(upcoming, key = { it.second.person.userId }) { (_, e, _) ->
                  Column {
                    Row(
                        Modifier.fillMaxWidth().clickable { vm.push(Route.PersonRoute(e.person.userId)) }.padding(vertical = 10.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        Avatar(e.profile?.avatar, e.name)
                        Column {
                            Text(e.name, style = MaterialTheme.typography.titleMedium)
                            Text(
                                fmtBirthday(e.person.birthdayMonth!!, e.person.birthdayDay!!, e.person.birthdayYear, zone),
                                style = MaterialTheme.typography.bodySmall,
                            )
                        }
                    }
                    HorizontalDivider()
                  }
                }
            }
        }
    }
}
