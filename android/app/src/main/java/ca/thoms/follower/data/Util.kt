// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

package ca.thoms.follower.data

import java.time.DateTimeException
import java.time.Instant
import java.time.LocalDate
import java.time.Month
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle
import java.time.format.TextStyle
import java.time.temporal.ChronoUnit
import java.util.Locale

val STATUSES: LinkedHashMap<String, String> = linkedMapOf(
    "friend" to "Friend",
    "unfriended" to "I unfriended them",
    "unfriended_me" to "They unfriended me",
    "blocked" to "I blocked them",
    "blocked_me" to "They blocked me",
    "acquaintance" to "Acquaintance",
    "other" to "Other",
)

fun statusLabel(key: String) = STATUSES[key] ?: key

fun normTag(t: String): String = t.trim().lowercase().replace(Regex("\\s+"), "-").take(32)

fun parseIds(text: String?): List<Long> =
    Regex("\\d{15,20}").findAll(text ?: "").mapNotNull { it.value.toLongOrNull() }.distinct().toList()

fun humanSize(bytes: Long): String {
    var n = bytes.toDouble()
    for (unit in listOf("B", "KB", "MB", "GB")) {
        if (n < 1024 || unit == "GB") return if (unit == "B") "%.0f B".format(n) else "%.1f %s".format(n, unit)
        n /= 1024
    }
    return "$bytes B"
}

fun parseBirthday(text: String): Triple<Int?, Int?, Int?> {
    val t = text.trim()
    if (t.isEmpty()) return Triple(null, null, null)
    val m = Regex("(?:(\\d{4})-)?(\\d{1,2})-(\\d{1,2})").matchEntire(t)
        ?: throw IllegalArgumentException("bad format")
    val y = m.groupValues[1].takeIf { it.isNotEmpty() }?.toInt()
    val mo = m.groupValues[2].toInt()
    val d = m.groupValues[3].toInt()
    try {
        LocalDate.of(y ?: 2000, mo, d)
    } catch (e: DateTimeException) {
        throw IllegalArgumentException("bad date")
    }
    return Triple(y, mo, d)
}

fun nextBirthday(mo: Int, d: Int, today: LocalDate): LocalDate {
    for (yr in listOf(today.year, today.year + 1)) {
        val c = try {
            LocalDate.of(yr, mo, d)
        } catch (e: DateTimeException) {
            LocalDate.of(yr, 2, 28)
        }
        if (!c.isBefore(today)) return c
    }
    throw IllegalStateException("unreachable")
}

fun fmtBirthday(mo: Int, d: Int, y: Int?, zone: ZoneId): String {
    val today = LocalDate.now(zone)
    val nxt = nextBirthday(mo, d, today)
    val days = ChronoUnit.DAYS.between(today, nxt)
    val whenText = if (days == 0L) "today" else "in $days day${if (days != 1L) "s" else ""}"
    val age = if (y != null) " · turns ${nxt.year - y}" else ""
    return "${Month.of(mo).getDisplayName(TextStyle.FULL, Locale.getDefault())} $d$age ($whenText)"
}

fun birthdayInput(mo: Int?, d: Int?, y: Int?): String {
    if (mo == null || d == null) return ""
    return (if (y != null) "$y-" else "") + "%02d-%02d".format(mo, d)
}

fun Instant.pretty(zone: ZoneId): String =
    DateTimeFormatter.ofLocalizedDateTime(FormatStyle.MEDIUM, FormatStyle.SHORT).withZone(zone).format(this)

fun Instant.prettyDate(zone: ZoneId): String =
    DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withZone(zone).format(this)
