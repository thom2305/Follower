// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Collections.ObjectModel;
using Follower.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace Follower;

public sealed partial class PeoplePage : Page
{
    readonly ObservableCollection<Row> rows = new();
    readonly List<long> visible = new();
    List<string> tagNames = new();
    bool syncing;

    public PeoplePage()
    {
        InitializeComponent();
        NavigationCacheMode = Microsoft.UI.Xaml.Navigation.NavigationCacheMode.Required;
        List.ItemsSource = rows;

        RelationBox.Items.Add("All statuses");
        foreach (var s in Util.Statuses) RelationBox.Items.Add(s.Label);
        RelationBox.SelectedIndex = 0;
        TagBox.Items.Add("All tags");
        TagBox.SelectedIndex = 0;

        AppState.Changed += OnChanged;
        OnChanged();
    }

    void OnChanged()
    {
        SyncTags();
        Render();
    }

    void SyncTags()
    {
        var names = AppState.People.SelectMany(p => p.Person.Tags).GroupBy(t => t)
            .OrderByDescending(g => g.Count()).ThenBy(g => g.Key, StringComparer.Ordinal).Select(g => g.Key).ToList();
        if (names.SequenceEqual(tagNames)) return;
        var prev = TagBox.SelectedIndex > 0 && TagBox.SelectedIndex - 1 < tagNames.Count ? tagNames[TagBox.SelectedIndex - 1] : null;
        syncing = true;
        TagBox.Items.Clear();
        TagBox.Items.Add("All tags");
        foreach (var n in names) TagBox.Items.Add("#" + n);
        tagNames = names;
        var idx = prev is null ? -1 : names.IndexOf(prev);
        TagBox.SelectedIndex = idx >= 0 ? idx + 1 : 0;
        syncing = false;
    }

    void Render()
    {
        var q = (SearchBox.Text ?? "").Trim().ToLowerInvariant();
        var relIdx = RelationBox.SelectedIndex;
        string? relKey = relIdx > 0 ? Util.Statuses[relIdx - 1].Key : null;
        var tagIdx = TagBox.SelectedIndex;
        string? tagSel = tagIdx > 0 && tagIdx - 1 < tagNames.Count ? tagNames[tagIdx - 1] : null;

        rows.Clear();
        visible.Clear();
        foreach (var e in AppState.People)
        {
            var p = e.Person;
            var ok = (relKey is null || p.Relation == relKey)
                && (tagSel is null || p.Tags.Contains(tagSel))
                && (q.Length == 0
                    || e.Name.ToLowerInvariant().Contains(q)
                    || (e.Profile?.Username.ToLowerInvariant().Contains(q) ?? false)
                    || p.UserId.ToString().Contains(q)
                    || (p.Notes?.ToLowerInvariant().Contains(q) ?? false)
                    || (p.MetContext?.ToLowerInvariant().Contains(q) ?? false));
            if (!ok) continue;

            var sub = (e.Profile is { } pr ? $"@{pr.Username} · " : "") + Util.StatusLabel(p.Relation);
            if (p.Tags.Count > 0) sub += "\n" + string.Join(" ", p.Tags.Select(t => "#" + t));
            rows.Add(new Row { Id = p.UserId, Name = e.Name, Subtitle = sub, Avatar = Ui.Img(e.Profile?.Avatar, 80) });
            visible.Add(p.UserId);
        }

        CountText.Text = $"People ({rows.Count})";
        EmptyText.Visibility = rows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
        EmptyText.Text = AppState.Loading && AppState.People.Count == 0 ? "Loading…"
            : AppState.People.Count == 0 ? "Nobody on your list yet. Use Lookup to add people."
            : "Nobody matches that.";
        Bar.Visibility = AppState.Progress is not null || AppState.Loading ? Visibility.Visible : Visibility.Collapsed;
        UpdateAllButton.Content = AppState.Progress ?? "Update all";
        UpdateAllButton.IsEnabled = AppState.Progress is null && rows.Count > 0;
    }

    void Filter_Changed(AutoSuggestBox sender, AutoSuggestBoxTextChangedEventArgs args)
    {
        if (args.Reason == AutoSuggestionBoxTextChangeReason.UserInput) Render();
    }
    void Search_Submitted(AutoSuggestBox sender, AutoSuggestBoxQuerySubmittedEventArgs args) => Render();
    void Filter_SelectionChanged(object sender, SelectionChangedEventArgs e) { if (!syncing) Render(); }

    void List_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is Row r) Frame.Navigate(typeof(PersonPage), r.Id);
    }

    async void UpdateAll_Click(object sender, RoutedEventArgs e) => await AppState.RefreshAllAsync(visible.ToList());
    async void Reload_Click(object sender, RoutedEventArgs e) => await AppState.LoadPeopleAsync();
}
