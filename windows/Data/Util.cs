// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Globalization;
using System.Text.RegularExpressions;

namespace Follower.Data;

public static class Util
{
    public static readonly (string Key, string Label)[] Statuses =
    {
        ("friend", "Friend"),
        ("unfriended", "I unfriended them"),
        ("unfriended_me", "They unfriended me"),
        ("blocked", "I blocked them"),
        ("blocked_me", "They blocked me"),
        ("acquaintance", "Acquaintance"),
        ("other", "Other"),
    };

    public static string StatusLabel(string key) =>
        Statuses.FirstOrDefault(s => s.Key == key).Label ?? key;

    public static string Enc(string s) => Uri.EscapeDataString(s);

    public static string NormTag(string t)
    {
        var s = Regex.Replace(t.Trim().ToLowerInvariant(), @"\s+", "-");
        return s.Length > 32 ? s[..32] : s;
    }

    static readonly Regex IdRe = new(@"\d{15,20}", RegexOptions.Compiled);

    public static List<long> ParseIds(string? text)
    {
        var seen = new HashSet<long>();
        var list = new List<long>();
        foreach (Match m in IdRe.Matches(text ?? ""))
            if (long.TryParse(m.Value, out var v) && seen.Add(v)) list.Add(v);
        return list;
    }

    public static string HumanSize(long bytes)
    {
        double n = bytes;
        foreach (var unit in new[] { "B", "KB", "MB", "GB" })
        {
            if (n < 1024 || unit == "GB")
                return unit == "B" ? $"{n:0} B" : $"{n:0.0} {unit}";
            n /= 1024;
        }
        return $"{bytes} B";
    }

    public static TimeZoneInfo Zone(string name)
    {
        try { return TimeZoneInfo.FindSystemTimeZoneById(name.Trim()); }
        catch { return TimeZoneInfo.Local; }
    }

    public static DateTimeOffset? ParseIso(string s) =>
        DateTimeOffset.TryParse(s.Trim(), CultureInfo.InvariantCulture,
            DateTimeStyles.AssumeUniversal | DateTimeStyles.AdjustToUniversal, out var d) ? d : null;

    public static string Pretty(this DateTimeOffset d, TimeZoneInfo tz) =>
        TimeZoneInfo.ConvertTime(d, tz).ToString("MMM d, yyyy, h:mm tt", CultureInfo.CurrentCulture);

    public static string PrettyDate(this DateTimeOffset d, TimeZoneInfo tz) =>
        TimeZoneInfo.ConvertTime(d, tz).ToString("MMM d, yyyy", CultureInfo.CurrentCulture);

    public static string PrettyStr(string s, TimeZoneInfo tz) => ParseIso(s) is { } d ? d.Pretty(tz) : s;

    public readonly record struct BirthdayParts(int? Year, int? Month, int? Day);

    static readonly Regex BdRe = new(@"^(?:(\d{4})-)?(\d{1,2})-(\d{1,2})$", RegexOptions.Compiled);

    public static BirthdayParts? ParseBirthday(string text)
    {
        var t = text.Trim();
        if (t.Length == 0) return new BirthdayParts(null, null, null);
        var m = BdRe.Match(t);
        if (!m.Success) return null;
        int? y = m.Groups[1].Success ? int.Parse(m.Groups[1].Value) : null;
        int mo = int.Parse(m.Groups[2].Value), d = int.Parse(m.Groups[3].Value);
        var yy = y ?? 2000;
        if (yy < 1 || mo < 1 || mo > 12 || d < 1 || d > DateTime.DaysInMonth(yy, mo)) return null;
        return new BirthdayParts(y, mo, d);
    }

    public static string BirthdayInput(int? mo, int? d, int? y) =>
        mo is null || d is null ? "" : (y is { } yy ? $"{yy}-" : "") + $"{mo:00}-{d:00}";

    public static DateOnly TodayIn(TimeZoneInfo tz) => DateOnly.FromDateTime(TimeZoneInfo.ConvertTime(DateTimeOffset.UtcNow, tz).DateTime);

    public static DateOnly NextBirthday(int mo, int d, DateOnly today)
    {
        foreach (var yr in new[] { today.Year, today.Year + 1 })
        {
            DateOnly? cand = d <= DateTime.DaysInMonth(yr, mo)
                ? new DateOnly(yr, mo, d)
                : (mo == 2 && d == 29 ? new DateOnly(yr, 2, 28) : null);
            if (cand is { } c && c >= today) return c;
        }
        return today;
    }

    public static int DaysUntilBirthday(int mo, int d, TimeZoneInfo tz)
    {
        var today = TodayIn(tz);
        return NextBirthday(mo, d, today).DayNumber - today.DayNumber;
    }

    public static string FmtBirthday(int mo, int d, int? y, TimeZoneInfo tz)
    {
        var today = TodayIn(tz);
        var nxt = NextBirthday(mo, d, today);
        var days = nxt.DayNumber - today.DayNumber;
        var when = days == 0 ? "today" : $"in {days} day{(days != 1 ? "s" : "")}";
        var age = y is { } yy ? $" · turns {nxt.Year - yy}" : "";
        var names = CultureInfo.CurrentCulture.DateTimeFormat.MonthNames;
        var name = mo >= 1 && mo <= 12 ? names[mo - 1] : mo.ToString();
        return $"{name} {d}{age} ({when})";
    }
}
