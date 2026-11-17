// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Text.Json.Nodes;

namespace Follower.Data;

public sealed class Repo(Api api)
{
    static JsonArray IdArray(IEnumerable<long> ids) => new(ids.Select(i => (JsonNode)JsonValue.Create(i.ToString())!).ToArray());

    public async Task<List<PersonEntry>> ListPeopleAsync()
    {
        var o = await api.GetAsync("/api/people");
        var list = new List<PersonEntry>();
        foreach (var p in o.Objs("people"))
            if (Person.From(p) is { } person)
                list.Add(new PersonEntry(person, p.Obj("profile") is { } pr ? Profile.From(pr) : null));
        return list;
    }

    public async Task<(Person? Person, Profile? Profile)> GetDetailAsync(long id)
    {
        var o = await api.GetAsync($"/api/people/{id}");
        return (Person.From(o.Obj("person")), o.Obj("profile") is { } pr ? Profile.From(pr) : null);
    }

    public async Task<Person?> GetPersonAsync(long id) => (await GetDetailAsync(id)).Person;

    public Task EnsurePersonAsync(long id) => api.PostAsync($"/api/people/{id}");

    public Task SetRelationAsync(long id, string relation) =>
        api.PostAsync($"/api/people/{id}", new JsonObject { ["relation"] = relation });

    public Task UpdateDetailsAsync(long id, string? notes, string? met, int? year, int? month, int? day) =>
        api.PostAsync($"/api/people/{id}", new JsonObject
        {
            ["notes"] = notes ?? "",
            ["met_context"] = met ?? "",
            ["birthday"] = Util.BirthdayInput(month, day, year),
        });

    public Task AddTagAsync(long id, string tag) =>
        api.PostAsync($"/api/people/{id}/tags", new JsonObject { ["tag"] = tag });

    public Task RemoveTagAsync(long id, string tag) => api.DeleteAsync($"/api/people/{id}/tags/{Util.Enc(tag)}");

    public Task RemovePersonAsync(long id) => api.DeleteAsync($"/api/people/{id}");

    public Task SaveManyAsync(IEnumerable<long> ids) =>
        api.PostAsync("/api/people/save", new JsonObject { ["ids"] = IdArray(ids) });

    public async Task<List<TagCount>> AllTagsAsync()
    {
        var o = await api.GetAsync("/api/tags");
        return o.Objs("tags").Where(t => t.Str("tag") is not null)
                .Select(t => new TagCount(t.Str("tag")!, (int)(t.Int("count") ?? 0))).ToList();
    }

    public async Task<(int People, int Archive)> CountsAsync()
    {
        var o = await api.GetAsync("/api/ping");
        return ((int)(o.Int("people") ?? 0), (int)(o.Int("archive") ?? 0));
    }

    static SnapResult SnapOf(JsonObject o) => new(
        o.Bool("found") && o.Obj("profile") is { } p ? Profile.From(p) : null,
        ChangeParser.Parse(o.Obj("changes")), o.Bool("is_first"));

    public async Task<SnapResult> TakeSnapshotAsync(long id) => SnapOf(await api.PostAsync($"/api/people/{id}/refresh"));

    public async Task<PersonHistory> HistoryAsync(long id)
    {
        var o = await api.GetAsync($"/api/people/{id}/history");
        return new PersonHistory(
            o.Objs("statuses").Select(s => new StatusEntry(s.Str("status") ?? "", s.Date("at") ?? DateTimeOffset.UtcNow)).ToList(),
            o.Objs("profile_changes").Select(c => new ProfileChange(
                c.Date("at") ?? DateTimeOffset.UtcNow, ChangeParser.Parse(c.Obj("changes")), c.Bool("is_first"))).ToList());
    }

    public async Task<List<LookupEntry>> LookupAsync(IEnumerable<long> ids, bool save, string? relation, string? tag)
    {
        var body = new JsonObject { ["ids"] = IdArray(ids), ["save"] = save };
        if (relation is not null) body["relation"] = relation;
        if (tag is not null) body["tag"] = tag;
        var o = await api.PostAsync("/api/lookup", body);
        var list = new List<LookupEntry>();
        foreach (var r in o.Objs("results"))
        {
            if (r.Id64("user_id") is not { } uid) continue;
            var s = SnapOf(r);
            list.Add(new LookupEntry { UserId = uid, Profile = s.Profile, Changes = s.Changes, Person = Person.From(r.Obj("person")) });
        }
        return list;
    }

    public async Task<List<ArchiveMsg>> ArchiveSearchAsync(string text, string author)
    {
        var o = await api.GetAsync($"/api/archive?text={Util.Enc(text)}&author={Util.Enc(author)}");
        return o.Objs("messages").Select(ArchiveMsg.From).OfType<ArchiveMsg>().ToList();
    }

    public async Task<ArchiveMsg?> ArchiveRandomAsync(string author)
    {
        var o = await api.GetAsync($"/api/archive/random?author={Util.Enc(author)}");
        return o.Obj("message") is { } m ? ArchiveMsg.From(m) : null;
    }

    public async Task<bool> ArchiveDeleteAsync(long messageId) => (await api.DeleteAsync($"/api/archive/{messageId}")).Bool("deleted");

    public async Task<List<AuthorChoice>> AuthorsAsync()
    {
        var o = await api.GetAsync("/api/archive/authors");
        return o.Objs("authors").Where(a => a.Id64("id") is not null)
                .Select(a => new AuthorChoice(a.Id64("id")!.Value, a.Str("name") ?? "?", (int)(a.Int("count") ?? 0))).ToList();
    }

    public Task<byte[]> LoadAttachmentAsync(long fileId) => api.BytesAsync(HttpMethod.Get, $"/api/archive/files/{fileId}");
}
