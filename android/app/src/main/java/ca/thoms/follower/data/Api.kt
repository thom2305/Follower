// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

package ca.thoms.follower.data

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLEncoder

class ApiException(message: String) : Exception(message)
class Api(private val config: () -> Config) {

    suspend fun get(path: String) = json("GET", path)
    suspend fun post(path: String, body: JSONObject = JSONObject()) = json("POST", path, body)
    suspend fun delete(path: String) = json("DELETE", path)

    private suspend fun json(method: String, path: String, body: JSONObject? = null): JSONObject =
        JSONObject(String(bytes(method, path, body), Charsets.UTF_8))

    suspend fun bytes(method: String, path: String, body: JSONObject? = null): ByteArray =
        withContext(Dispatchers.IO) { blocking(method, path, body) }

    private fun blocking(method: String, path: String, body: JSONObject?): ByteArray {
        val c = config()
        if (!c.ready) throw ApiException("Set the server URL and API token in Settings first.")
        var base = c.url.trim().trimEnd('/')
        if (!base.startsWith("http://") && !base.startsWith("https://")) base = "https://$base"

        val conn = try {
            URL(base + path).openConnection() as HttpURLConnection
        } catch (e: IOException) {
            throw ApiException("That server URL doesn't look valid.")
        }
        try {
            conn.requestMethod = method
            conn.connectTimeout = 10_000
            conn.readTimeout = 60_000
            conn.setRequestProperty("Authorization", "Bearer ${c.token.trim()}")
            conn.setRequestProperty("Accept", "application/json")
            val payload = body ?: if (method == "POST") JSONObject() else null
            if (payload != null) {
                conn.doOutput = true
                conn.setRequestProperty("Content-Type", "application/json; charset=utf-8")
                conn.outputStream.use { it.write(payload.toString().toByteArray(Charsets.UTF_8)) }
            }
            val code = conn.responseCode
            if (code in 200..299) return conn.inputStream.use { it.readBytes() }
            val err = conn.errorStream?.use { it.readBytes() }?.toString(Charsets.UTF_8).orEmpty()
            val msg = runCatching { JSONObject(err).getString("error") }.getOrNull()
            throw ApiException(
                when {
                    code == 401 -> "Wrong API token (401)."
                    msg != null -> msg
                    else -> "Server returned HTTP $code."
                }
            )
        } catch (e: ApiException) {
            throw e
        } catch (e: IOException) {
            throw ApiException("Couldn't reach the server: ${e.message ?: e.javaClass.simpleName}")
        } finally {
            conn.disconnect()
        }
    }
}

fun enc(s: String): String = URLEncoder.encode(s, "UTF-8").replace("+", "%20")
