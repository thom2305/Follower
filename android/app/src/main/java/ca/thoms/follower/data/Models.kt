// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

package ca.thoms.follower.data

import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant

typealias Changes = Map<String, Pair<String, String>>

fun JSONObject.str(k: String): String? = if (isNull(k)) null else getString(k)
fun JSONObject.intOrNull(k: String): Int? = if (isNull(k)) null else getInt(k)
fun <T> JSONArray.mapObjects(f: (JSONObject) -> T): List<T> = List(length()) { f(getJSONObject(it)) }
fun JSONArray.strings(): List<String> = List(length()) { getString(it) }

fun parseChanges(o: JSONObject?): Changes {
    if (o == null) return emptyMap()
    val out = LinkedHashMap<String, Pair<String, String>>()
    for (k in o.keys()) {
        val a = o.getJSONArray(k)
        out[k] = a.optString(0) to a.optString(1)
    }
    return out
}

data class Profile(
    val username: String,
    val globalName: String?,
    val avatar: String?,
    val banner: String?,
    val accentColor: Int?,
    val avatarDecoration: String?,
    val bot: Boolean,
    val badges: List<String>,
    val createdAt: String,
) {
    val displayName: String get() = globalName ?: username

    companion object {
        fun fromJson(o: JSONObject): Profile = Profile(
            username = o.optString("username"),
            globalName = o.str("global_name"),
            avatar = o.str("avatar"),
            banner = o.str("banner"),
            accentColor = o.intOrNull("accent_color"),
            avatarDecoration = o.str("avatar_decoration"),
            bot = o.optBoolean("bot", false),
            badges = o.optJSONArray("badges")?.strings() ?: emptyList(),
            createdAt = o.optString("created_at"),
        )
    }
}

data class Person(
    val userId: Long,
    val relation: String,
    val notes: String?,
    val metContext: String?,
    val birthdayMonth: Int?,
    val birthdayDay: Int?,
    val birthdayYear: Int?,
    val firstSeen: Instant,
    val updatedAt: Instant,
    val tags: List<String> = emptyList(),
) {
    companion object {
        fun fromJson(o: JSONObject) = Person(
            userId = o.getString("user_id").toLong(),
            relation = o.getString("relation"),
            notes = o.str("notes"),
            metContext = o.str("met_context"),
            birthdayMonth = o.intOrNull("birthday_month"),
            birthdayDay = o.intOrNull("birthday_day"),
            birthdayYear = o.intOrNull("birthday_year"),
            firstSeen = Instant.parse(o.getString("first_seen")),
            updatedAt = Instant.parse(o.getString("updated_at")),
            tags = o.optJSONArray("tags")?.strings() ?: emptyList(),
        )
    }
}

data class PersonEntry(val person: Person, val profile: Profile?) {
    val name: String get() = profile?.displayName ?: person.userId.toString()
}

data class SnapResult(val profile: Profile?, val changes: Changes, val isFirst: Boolean)

data class StatusEntry(val status: String, val at: Instant)
data class ProfileChange(val at: Instant, val changes: Changes, val isFirst: Boolean)
data class PersonHistory(val statuses: List<StatusEntry>, val profileChanges: List<ProfileChange>)

data class Attachment(
    val id: Long,
    val filename: String,
    val contentType: String?,
    val size: Long,
    val url: String?,
    val compressed: Boolean,
    val storedSize: Long,
    val stored: Boolean,
)

data class ArchiveMsg(
    val messageId: Long,
    val authorId: Long,
    val authorName: String,
    val authorAvatar: String?,
    val content: String?,
    val jumpUrl: String?,
    val messageTs: Instant,
    val savedAt: Instant,
    val attachments: List<Attachment>,
) {
    companion object {
        fun fromJson(o: JSONObject) = ArchiveMsg(
            messageId = o.getString("message_id").toLong(),
            authorId = o.getString("author_id").toLong(),
            authorName = o.getString("author_name"),
            authorAvatar = o.str("author_avatar"),
            content = o.str("content"),
            jumpUrl = o.str("jump_url"),
            messageTs = Instant.parse(o.getString("message_ts")),
            savedAt = Instant.parse(o.getString("saved_at")),
            attachments = o.getJSONArray("attachments").mapObjects {
                Attachment(
                    id = it.getLong("id"),
                    filename = it.getString("filename"),
                    contentType = it.str("content_type"),
                    size = it.getLong("size"),
                    url = it.str("url"),
                    compressed = it.getBoolean("compressed"),
                    storedSize = it.getLong("stored_size"),
                    stored = it.getBoolean("stored"),
                )
            },
        )
    }
}

data class AuthorChoice(val id: Long, val name: String, val count: Int)

data class LookupEntry(val userId: Long, val profile: Profile?, val changes: Changes, val person: Person?)
