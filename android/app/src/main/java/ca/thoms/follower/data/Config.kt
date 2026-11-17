// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

package ca.thoms.follower.data

import android.content.Context
import java.time.ZoneId

data class Config(
    val url: String = "",
    val token: String = "",
    val timezone: String = "America/Toronto",
) {
    val ready: Boolean get() = url.isNotBlank() && token.isNotBlank()
    val zone: ZoneId get() = runCatching { ZoneId.of(timezone) }.getOrDefault(ZoneId.systemDefault())
}

class ConfigStore(ctx: Context) {
    private val p = ctx.getSharedPreferences("follower", Context.MODE_PRIVATE)

    fun load() = Config(
        url = p.getString("url", "") ?: "",
        token = p.getString("api_token", "") ?: "",
        timezone = p.getString("tz", "America/Toronto") ?: "America/Toronto",
    )

    fun save(c: Config) {
        p.edit()
            .putString("url", c.url.trim())
            .putString("api_token", c.token.trim())
            .putString("tz", c.timezone.trim())
            .apply()
    }
}
