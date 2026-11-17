// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Text.Json.Nodes;

namespace Follower.Data;

public sealed record Change(string Key, string From, string To);

public sealed record Profile(
    string Username, string? GlobalName, string? Avatar, string? Banner,
    bool Bot, List<string> Badges, string CreatedAt)
{
    public string DisplayName => GlobalName ?? Username;

    public static Profile From(JsonNode o) => new(
        o.Str("username") ?? "", o.Str("global_name"), o.Str("avatar"), o.Str("banner"),
        o.Bool("bot"), o.Strings("badges"), o.Str("created_at") ?? "");
}

public sealed record Person(
    long UserId, string Relation, string? Notes, string? MetContext,
    int? BirthdayMonth, int? BirthdayDay, int? BirthdayYear,
    DateTimeOffset FirstSeen, DateTimeOffset UpdatedAt, List<string> Tags)
{
    public static Person? From(JsonNode? o)
    {
        if (o?.Id64("user_id") is not { } id) return null;
        return new Person(
            id, o.Str("relation") ?? "other", o.Str("notes"), o.Str("met_context"),
            (int?)o.Int("birthday_month"), (int?)o.Int("birthday_day"), (int?)o.Int("birthday_year"),
            o.Date("first_seen") ?? DateTimeOffset.UtcNow, o.Date("updated_at") ?? DateTimeOffset.UtcNow,
            o.Strings("tags"));
    }
}

public sealed record PersonEntry(Person Person, Profile? Profile)
{
    public string Name => Profile?.DisplayName ?? Person.UserId.ToString();
}

public sealed record SnapResult(Profile? Profile, List<Change> Changes, bool IsFirst);
public sealed record StatusEntry(string Status, DateTimeOffset At);
public sealed record ProfileChange(DateTimeOffset At, List<Change> Changes, bool IsFirst);
public sealed record PersonHistory(List<StatusEntry> Statuses, List<ProfileChange> ProfileChanges);

public sealed record Attachment(
    long Id, string Filename, string? ContentType, long Size, string? Url,
    bool Compressed, long StoredSize, bool Stored);

public sealed record ArchiveMsg(
    long MessageId, long AuthorId, string AuthorName, string? AuthorAvatar, string? Content,
    string? JumpUrl, DateTimeOffset MessageTs, DateTimeOffset SavedAt, List<Attachment> Attachments)
{
    public static ArchiveMsg? From(JsonNode o)
    {
        if (o.Id64("message_id") is not { } mid) return null;
        var atts = new List<Attachment>();
        foreach (var a in o.Objs("attachments"))
        {
            if (a.Id64("id") is not { } aid) continue;
            atts.Add(new Attachment(aid, a.Str("filename") ?? "file", a.Str("content_type"),
                a.Id64("size") ?? 0, a.Str("url"), a.Bool("compressed"),
                a.Id64("stored_size") ?? 0, a.Bool("stored")));
        }
        return new ArchiveMsg(mid, o.Id64("author_id") ?? 0, o.Str("author_name") ?? "?", o.Str("author_avatar"),
            o.Str("content"), o.Str("jump_url"), o.Date("message_ts") ?? DateTimeOffset.UtcNow,
            o.Date("saved_at") ?? DateTimeOffset.UtcNow, atts);
    }
}

public sealed record AuthorChoice(long Id, string Name, int Count);
public sealed record TagCount(string Tag, int Count);

public sealed class LookupEntry
{
    public long UserId { get; init; }
    public Profile? Profile { get; set; }
    public List<Change> Changes { get; set; } = new();
    public Person? Person { get; set; }
}

public static class ChangeParser
{
    static string Stringify(JsonNode? x) => x switch
    {
        null => "",
        JsonValue v when v.TryGetValue<string>(out var s) => s,
        _ => x.ToJsonString(),
    };

    public static List<Change> Parse(JsonObject? o)
    {
        var list = new List<Change>();
        if (o is null) return list;
        foreach (var kv in o.OrderBy(k => k.Key, StringComparer.Ordinal))
        {
            var a = kv.Value as JsonArray;
            list.Add(new Change(kv.Key, Stringify(a is { Count: > 0 } ? a[0] : null), Stringify(a is { Count: > 1 } ? a[1] : null)));
        }
        return list;
    }
}
