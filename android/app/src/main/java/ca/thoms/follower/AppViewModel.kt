// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

package ca.thoms.follower

import android.app.Application
import android.net.Uri
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.List
import androidx.compose.material.icons.filled.DateRange
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import ca.thoms.follower.data.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

enum class Tab(val label: String, val icon: ImageVector) {
    People("People", Icons.Default.Person),
    Lookup("Lookup", Icons.Default.Search),
    Birthdays("Birthdays", Icons.Default.DateRange),
    Archive("Archive", Icons.AutoMirrored.Filled.List),
    Settings("Settings", Icons.Default.Settings),
}

sealed interface Route {
    data class PersonRoute(val userId: Long) : Route
    data class ArchiveRoute(val msg: ArchiveMsg) : Route
}

data class PersonDetail(
    val userId: Long,
    val profile: Profile?,
    val changes: Changes,
    val person: Person?,
    val refreshing: Boolean = false,
    val gone: Boolean = false,
)

class AppViewModel(app: Application) : AndroidViewModel(app) {
    private val store = ConfigStore(app)
    var config by mutableStateOf(store.load()); private set

    private val api = Api { config }
    val repo = Repo(api)

    val messages = MutableSharedFlow<String>(extraBufferCapacity = 8)

    var tab by mutableStateOf(if (config.ready) Tab.People else Tab.Settings)
    val backStack = mutableStateListOf<Route>()
    fun push(r: Route) {
        backStack.add(r)
        if (r is Route.PersonRoute) openPerson(r.userId)
    }
    fun pop() {
        if (backStack.isNotEmpty()) backStack.removeAt(backStack.lastIndex)
    }

    var people by mutableStateOf<List<PersonEntry>>(emptyList()); private set
    var peopleLoading by mutableStateOf(false); private set
    var progress by mutableStateOf<String?>(null); private set
    var allTags by mutableStateOf<List<Pair<String, Int>>>(emptyList()); private set

    var detail by mutableStateOf<PersonDetail?>(null); private set
    var history by mutableStateOf<PersonHistory?>(null); private set

    var lookupResults by mutableStateOf<List<LookupEntry>>(emptyList()); private set
    var lookupBusy by mutableStateOf(false); private set

    var archiveResults by mutableStateOf<List<ArchiveMsg>?>(null); private set
    var archiveBusy by mutableStateOf(false); private set
    var authors by mutableStateOf<List<AuthorChoice>>(emptyList()); private set

    init {
        if (config.ready) loadPeople()
    }

    private fun bg(block: suspend () -> Unit): Job = viewModelScope.launch {
        try {
            block()
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            messages.tryEmit(e.message?.takeIf { it.isNotBlank() } ?: e.javaClass.simpleName)
        }
    }

    fun saveConfig(c: Config, test: Boolean) = bg {
        store.save(c)
        config = c
        if (test) {
            val (p, a) = repo.counts()
            messages.tryEmit("Connected: $p people, $a archived messages")
            loadPeopleNow()
        } else {
            messages.tryEmit("Saved")
        }
    }

    fun loadPeople() = bg { loadPeopleNow() }

    private suspend fun loadPeopleNow() {
        peopleLoading = true
        try {
            people = repo.listPeople()
            allTags = repo.allTags()
        } finally {
            peopleLoading = false
        }
    }

    fun refreshAll(ids: List<Long>) = bg {
        if (ids.isEmpty()) return@bg
        var changed = 0
        var gone = 0
        try {
            ids.forEachIndexed { i, id ->
                progress = "Refreshing ${i + 1}/${ids.size}…"
                val r = repo.takeSnapshot(id)
                if (r.profile == null) gone++ else if (r.changes.isNotEmpty()) changed++
            }
        } finally {
            progress = null
        }
        loadPeopleNow()
        messages.tryEmit("Refreshed ${ids.size} · $changed changed" + if (gone > 0) " · $gone not found" else "")
    }

    private fun openPerson(id: Long) {
        history = null
        detail = PersonDetail(id, null, emptyMap(), null, refreshing = true)
        bg {
            val (person, profile) = repo.getDetail(id)
            detail = PersonDetail(id, profile, emptyMap(), person, refreshing = true)
            refreshDetailNow(id)
        }
    }

    fun refreshDetail() {
        val id = detail?.userId ?: return
        bg { refreshDetailNow(id) }
    }

    private suspend fun refreshDetailNow(id: Long) {
        try {
            val r = repo.takeSnapshot(id)
            detail = detail?.takeIf { it.userId == id }?.let {
                it.copy(profile = r.profile ?: it.profile, changes = r.changes, gone = r.profile == null)
            }
        } catch (e: CancellationException) {
            throw e
        } catch (e: Throwable) {
            messages.tryEmit((e.message ?: "Couldn't refresh") + " (showing the last saved profile)")
        } finally {
            detail = detail?.takeIf { it.userId == id }?.copy(refreshing = false)
        }
    }

    private suspend fun afterPersonChange(id: Long) {
        val p = repo.getPerson(id)
        detail = detail?.takeIf { it.userId == id }?.copy(person = p)
        people = repo.listPeople()
        allTags = repo.allTags()
    }

    fun setRelation(id: Long, relation: String) = bg { repo.setRelation(id, relation); afterPersonChange(id) }
    fun addTag(id: Long, raw: String) = bg {
        raw.split(",").map(::normTag).filter { it.isNotEmpty() }.toSet().forEach { repo.addTag(id, it) }
        afterPersonChange(id)
    }
    fun removeTag(id: Long, tag: String) = bg { repo.removeTag(id, tag); afterPersonChange(id) }
    fun addToList(id: Long) = bg { repo.ensurePerson(id); afterPersonChange(id) }

    fun saveDetails(id: Long, notes: String, met: String, y: Int?, mo: Int?, d: Int?) = bg {
        repo.updateDetails(id, notes.trim().ifEmpty { null }, met.trim().take(60).ifEmpty { null }, y, mo, d)
        afterPersonChange(id)
    }

    fun removePerson(id: Long) = bg {
        repo.removePerson(id)
        pop()
        people = repo.listPeople()
        allTags = repo.allTags()
        messages.tryEmit("Removed from your list (profile snapshots kept).")
    }

    fun loadHistory(id: Long) = bg { history = repo.history(id) }

    fun lookup(ids: List<Long>, save: Boolean, relation: String?, tag: String?) = bg {
        lookupBusy = true
        try {
            val t = tag?.let(::normTag)?.takeIf { it.isNotEmpty() }
            lookupResults = repo.lookup(ids.take(25), save, relation, t)
            if (save) loadPeopleNow()
        } finally {
            lookupBusy = false
        }
    }

    fun saveLookupToList(ids: List<Long>) = bg {
        repo.saveMany(ids)
        lookupResults = lookupResults.map { if (it.userId in ids) it.copy(person = repo.getPerson(it.userId)) else it }
        loadPeopleNow()
        messages.tryEmit("Saved ${ids.size} to your list")
    }

    fun loadAuthors() = bg { authors = repo.authors() }

    fun searchArchive(text: String, author: String) = bg {
        archiveBusy = true
        try {
            archiveResults = repo.archiveSearch(text, author)
        } finally {
            archiveBusy = false
        }
    }

    fun randomArchive(author: String) = bg {
        archiveBusy = true
        try {
            val m = repo.archiveRandom(author)
            if (m == null) messages.tryEmit("Archive is empty.") else push(Route.ArchiveRoute(m))
        } finally {
            archiveBusy = false
        }
    }

    fun deleteArchive(id: Long) = bg {
        repo.archiveDelete(id)
        pop()
        archiveResults = archiveResults?.filterNot { it.messageId == id }
        messages.tryEmit("Deleted (including stored attachments).")
    }

    suspend fun attachmentBytes(fileId: Long): ByteArray? = try {
        repo.loadAttachment(fileId)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Throwable) {
        messages.tryEmit(e.message ?: "Couldn't load attachment")
        null
    }

    fun saveAttachment(uri: Uri, fileId: Long) = bg {
        val bytes = repo.loadAttachment(fileId)
        withContext(Dispatchers.IO) {
            getApplication<Application>().contentResolver.openOutputStream(uri)?.use { it.write(bytes) }
        }
        messages.tryEmit("Saved to device")
    }
}
