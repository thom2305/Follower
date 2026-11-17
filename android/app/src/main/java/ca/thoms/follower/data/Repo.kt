// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

package ca.thoms.follower.data

import org.json.JSONArray
import org.json.JSONObject
import java.time.Instant

class Repo(private val api: Api) {
    suspend fun listPeople(): List<PersonEntry> =
        api.get("/api/people").getJSONArray("people").mapObjects {
            PersonEntry(Person.fromJson(it), it.optJSONObject("profile")?.let { Profile.fromJson(it) })
        }

    suspend fun getDetail(id: Long): Pair<Person?, Profile?> {
        val o = api.get("/api/people/$id")
        return o.optJSONObject("person")?.let { Person.fromJson(it) } to o.optJSONObject("profile")?.let { Profile.fromJson(it) }
    }

    suspend fun getPerson(id: Long): Person? = getDetail(id).first

    suspend fun ensurePerson(id: Long) {
        api.post("/api/people/$id")
    }

    suspend fun setRelation(id: Long, relation: String) {
        api.post("/api/people/$id", JSONObject().put("relation", relation))
    }

    suspend fun updateDetails(id: Long, notes: String?, met: String?, year: Int?, month: Int?, day: Int?) {
        api.post(
            "/api/people/$id",
            JSONObject()
                .put("notes", notes ?: "")
                .put("met_context", met ?: "")
                .put("birthday", birthdayInput(month, day, year)),
        )
    }

    suspend fun addTag(id: Long, tag: String) {
        api.post("/api/people/$id/tags", JSONObject().put("tag", tag))
    }

    suspend fun removeTag(id: Long, tag: String) {
        api.delete("/api/people/$id/tags/${enc(tag)}")
    }

    suspend fun removePerson(id: Long) {
        api.delete("/api/people/$id")
    }

    suspend fun saveMany(ids: List<Long>) {
        api.post("/api/people/save", JSONObject().put("ids", JSONArray(ids.map { it.toString() })))
    }

    suspend fun allTags(): List<Pair<String, Int>> =
        api.get("/api/tags").getJSONArray("tags").mapObjects { it.getString("tag") to it.getInt("count") }

    suspend fun counts(): Pair<Int, Int> {
        val o = api.get("/api/ping")
        return o.getInt("people") to o.getInt("archive")
    }

    private fun snapOf(o: JSONObject) = SnapResult(
        profile = if (o.getBoolean("found")) o.optJSONObject("profile")?.let { Profile.fromJson(it) } else null,
        changes = parseChanges(o.optJSONObject("changes")),
        isFirst = o.optBoolean("is_first", false),
    )

    suspend fun takeSnapshot(id: Long): SnapResult = snapOf(api.post("/api/people/$id/refresh"))

    suspend fun history(id: Long): PersonHistory {
        val o = api.get("/api/people/$id/history")
        return PersonHistory(
            statuses = o.getJSONArray("statuses").mapObjects { StatusEntry(it.getString("status"), Instant.parse(it.getString("at"))) },
            profileChanges = o.getJSONArray("profile_changes").mapObjects {
                ProfileChange(Instant.parse(it.getString("at")), parseChanges(it.optJSONObject("changes")), it.optBoolean("is_first"))
            },
        )
    }

    suspend fun lookup(ids: List<Long>, save: Boolean, relation: String?, tag: String?): List<LookupEntry> {
        val body = JSONObject()
            .put("ids", JSONArray(ids.map { it.toString() }))
            .put("save", save)
        if (relation != null) body.put("relation", relation)
        if (tag != null) body.put("tag", tag)
        return api.post("/api/lookup", body).getJSONArray("results").mapObjects {
            val s = snapOf(it)
            LookupEntry(it.getString("user_id").toLong(), s.profile, s.changes, it.optJSONObject("person")?.let { Person.fromJson(it) })
        }
    }

    suspend fun archiveSearch(text: String?, author: String?): List<ArchiveMsg> =
        api.get("/api/archive?text=${enc(text.orEmpty())}&author=${enc(author.orEmpty())}")
            .getJSONArray("messages").mapObjects { ArchiveMsg.fromJson(it) }

    suspend fun archiveRandom(author: String?): ArchiveMsg? =
        api.get("/api/archive/random?author=${enc(author.orEmpty())}").optJSONObject("message")?.let { ArchiveMsg.fromJson(it) }

    suspend fun archiveDelete(messageId: Long): Boolean =
        api.delete("/api/archive/$messageId").optBoolean("deleted", false)

    suspend fun authors(): List<AuthorChoice> =
        api.get("/api/archive/authors").getJSONArray("authors").mapObjects {
            AuthorChoice(it.getString("id").toLong(), it.getString("name"), it.getInt("count"))
        }

    suspend fun loadAttachment(fileId: Long): ByteArray = api.bytes("GET", "/api/archive/files/$fileId")
}
