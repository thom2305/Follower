// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using Follower.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Navigation;
using Windows.System;

namespace Follower;

public sealed partial class PersonPage : Page
{
    long id;
    Profile? profile;
    Person? person;
    List<Change> changes = new();
    bool gone;
    PersonHistory? history;
    bool suppress;

    public PersonPage()
    {
        InitializeComponent();
        Ui.Fit(Scroll, Column);
    }

    protected override async void OnNavigatedTo(NavigationEventArgs e)
    {
        id = (long)e.Parameter;
        foreach (var s in Util.Statuses) RelationBox.Items.Add(s.Label);
        Render();
        SetBusy(true);
        var ok = await AppState.RunAsync(async () =>
        {
            (person, profile) = await AppState.Repo.GetDetailAsync(id);
        });
        Render();
        if (ok) await RefreshAsync(); else SetBusy(false);
    }

    static void Fill(WrapPanel panel, IEnumerable<Button> buttons)
    {
        panel.Children.Clear();
        foreach (var b in buttons) panel.Children.Add(b);
    }

    void SetBusy(bool busy)
    {
        Ring.IsActive = busy;
        Ring.Visibility = busy ? Visibility.Visible : Visibility.Collapsed;
        RefreshButton.Visibility = busy ? Visibility.Collapsed : Visibility.Visible;
    }

    async Task RefreshAsync()
    {
        SetBusy(true);
        await AppState.RunAsync(async () =>
        {
            var r = await AppState.Repo.TakeSnapshotAsync(id);
            gone = r.Profile is null;
            if (r.Profile is not null) profile = r.Profile;
            changes = r.Changes;
        });
        SetBusy(false);
        Render();
    }

    async Task MutateAsync(Func<Repo, Task> op)
    {
        var ok = await AppState.RunAsync(async () =>
        {
            await op(AppState.Repo);
            person = await AppState.Repo.GetPersonAsync(id);
        });
        if (ok) { Render(); _ = AppState.LoadPeopleAsync(); }
    }

    void Render()
    {
        var zone = AppState.Config.Zone;
        var name = profile?.DisplayName ?? id.ToString();
        NameText.Text = name;
        UserText.Text = profile is null ? "" : "@" + profile.Username;
        UserText.Visibility = profile is null ? Visibility.Collapsed : Visibility.Visible;
        IdText.Text = id.ToString();
        Pic.DisplayName = name;
        Pic.ProfilePicture = Ui.Img(profile?.Avatar, 144);
        GoneBar.IsOpen = gone;

        PropsPanel.Children.Clear();
        PropsCard.Visibility = profile is null ? Visibility.Collapsed : Visibility.Visible;
        if (profile is { } p)
        {
            PropsPanel.Children.Add(Ui.Prop("Created", Util.PrettyStr(p.CreatedAt, zone)));
            PropsPanel.Children.Add(Ui.Prop("Badges", p.Badges.Count == 0 ? "none" : string.Join(", ", p.Badges)));
            if (p.Bot) PropsPanel.Children.Add(Ui.Prop("Bot", "yes"));
        }

        ChangesPanel.Children.Clear();
        ChangesCard.Visibility = changes.Count == 0 ? Visibility.Collapsed : Visibility.Visible;
        if (changes.Count > 0)
        {
            ChangesPanel.Children.Add(Ui.Heading("Changed since last check"));
            foreach (var c in changes)
                ChangesPanel.Children.Add(Ui.Prop(c.Key, $"{Ui.Trunc(c.From, 60)} → {Ui.Trunc(c.To, 60)}"));
        }

        var tracked = person is not null;
        RelationBox.Visibility = tracked ? Visibility.Visible : Visibility.Collapsed;
        TrackedText.Text = tracked ? $"Tracked since {person!.FirstSeen.PrettyDate(zone)}" : "Not on your list yet.";
        AddToListButton.Visibility = tracked ? Visibility.Collapsed : Visibility.Visible;
        if (person is not null)
        {
            suppress = true;
            var idx = Array.FindIndex(Util.Statuses, s => s.Key == person.Relation);
            RelationBox.SelectedIndex = idx >= 0 ? idx : Util.Statuses.Length - 1;
            suppress = false;
        }

        TagsCard.Visibility = DetailsCard.Visibility = tracked ? Visibility.Visible : Visibility.Collapsed;
        RemoveButton.Visibility = tracked ? Visibility.Visible : Visibility.Collapsed;
        if (person is { } pe)
        {
            Fill(TagsRepeater, pe.Tags.Select(t =>
            {
                var b = new Button { Content = $"#{t}  ✕", CornerRadius = new CornerRadius(14) };
                ToolTipService.SetToolTip(b, "Remove tag");
                b.Click += async (_, _) => await MutateAsync(r => r.RemoveTagAsync(id, t));
                return b;
            }));

            var suggestions = AppState.Tags.Select(t => t.Tag).Where(t => !pe.Tags.Contains(t)).Take(12).ToList();
            SuggestLabel.Visibility = SuggestRepeater.Visibility = suggestions.Count > 0 ? Visibility.Visible : Visibility.Collapsed;
            Fill(SuggestRepeater, suggestions.Select(t =>
            {
                var b = new Button { Content = "#" + t, CornerRadius = new CornerRadius(14) };
                b.Click += async (_, _) => await AddTagsAsync(t);
                return b;
            }));

            DetailsPanel.Children.Clear();
            if (pe.BirthdayMonth is { } mo && pe.BirthdayDay is { } d)
                DetailsPanel.Children.Add(Ui.Prop("Birthday", Util.FmtBirthday(mo, d, pe.BirthdayYear, zone)));
            if (!string.IsNullOrWhiteSpace(pe.MetContext)) DetailsPanel.Children.Add(Ui.Prop("Know them from", pe.MetContext!));
            if (!string.IsNullOrWhiteSpace(pe.Notes)) DetailsPanel.Children.Add(Ui.Prop("Notes", pe.Notes!));
        }

        HistoryPanel.Children.Clear();
        HistoryButton.Visibility = history is null ? Visibility.Visible : Visibility.Collapsed;
        if (history is { } h)
        {
            foreach (var s in h.Statuses)
                HistoryPanel.Children.Add(Ui.Prop("Relation: " + Util.StatusLabel(s.Status), s.At.Pretty(zone)));
            if (h.ProfileChanges.Count == 0) HistoryPanel.Children.Add(Ui.Muted("No snapshots yet."));
            foreach (var ch in h.ProfileChanges)
            {
                var lines = string.Join("\n", ch.Changes.Select(x => $"{x.Key}: {Ui.Trunc(x.From, 40)} → {Ui.Trunc(x.To, 40)}"));
                HistoryPanel.Children.Add(Ui.Prop(ch.At.Pretty(zone) + (ch.IsFirst ? " · first snapshot" : ""), lines));
            }
        }
    }

    async void Refresh_Click(object sender, RoutedEventArgs e) => await RefreshAsync();

    async void Relation_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (suppress || person is null || RelationBox.SelectedIndex < 0) return;
        var key = Util.Statuses[RelationBox.SelectedIndex].Key;
        if (key != person.Relation) await MutateAsync(r => r.SetRelationAsync(id, key));
    }

    async void AddToList_Click(object sender, RoutedEventArgs e) => await MutateAsync(r => r.EnsurePersonAsync(id));

    async Task AddTagsAsync(string raw)
    {
        var tags = raw.Split(',').Select(Util.NormTag).Where(t => t.Length > 0).Distinct().ToList();
        if (tags.Count == 0) return;
        await MutateAsync(async r => { foreach (var t in tags) await r.AddTagAsync(id, t); });
    }

    async void AddTag_Click(object sender, RoutedEventArgs e)
    {
        var text = TagBox.Text; TagBox.Text = "";
        await AddTagsAsync(text);
    }

    async void TagBox_KeyDown(object sender, Microsoft.UI.Xaml.Input.KeyRoutedEventArgs e)
    {
        if (e.Key != VirtualKey.Enter) return;
        e.Handled = true;
        var text = TagBox.Text; TagBox.Text = "";
        await AddTagsAsync(text);
    }

    async void History_Click(object sender, RoutedEventArgs e)
    {
        await AppState.RunAsync(async () => history = await AppState.Repo.HistoryAsync(id));
        Render();
    }

    async void Edit_Click(object sender, RoutedEventArgs e)
    {
        if (person is not { } p) return;
        var notes = new TextBox { Header = "Notes", AcceptsReturn = true, TextWrapping = TextWrapping.Wrap, MinHeight = 110, MaxLength = 1000, Text = p.Notes ?? "" };
        var met = new TextBox { Header = "Where you know them from", MaxLength = 60, PlaceholderText = "e.g. school, a Discord server", Text = p.MetContext ?? "" };
        var bday = new TextBox { Header = "Birthday", MaxLength = 10, PlaceholderText = "YYYY-MM-DD or MM-DD", Text = Util.BirthdayInput(p.BirthdayMonth, p.BirthdayDay, p.BirthdayYear) };
        var err = new TextBlock { Foreground = (Microsoft.UI.Xaml.Media.Brush)Application.Current.Resources["SystemFillColorCriticalBrush"], Visibility = Visibility.Collapsed };
        var panel = new StackPanel { Spacing = 12 };
        panel.Children.Add(notes); panel.Children.Add(met); panel.Children.Add(bday); panel.Children.Add(err);

        var dlg = new ContentDialog
        {
            XamlRoot = XamlRoot, Title = "Person details", Content = panel,
            PrimaryButtonText = "Save", CloseButtonText = "Cancel", DefaultButton = ContentDialogButton.Primary,
        };
        Util.BirthdayParts? parts = null;
        dlg.PrimaryButtonClick += (_, args) =>
        {
            parts = Util.ParseBirthday(bday.Text);
            if (parts is null)
            {
                err.Text = "Use 1999-03-05 or 03-05.";
                err.Visibility = Visibility.Visible;
                args.Cancel = true;
            }
        };
        if (await dlg.ShowAsync() != ContentDialogResult.Primary || parts is not { } bp) return;
        var n = notes.Text.Trim(); var m = met.Text.Trim();
        await MutateAsync(r => r.UpdateDetailsAsync(id, n.Length > 0 ? n : null, m.Length > 0 ? m : null, bp.Year, bp.Month, bp.Day));
    }

    async void Remove_Click(object sender, RoutedEventArgs e)
    {
        if (!await Ui.ConfirmAsync(XamlRoot, "Remove from list?", "Tags and details will be deleted. Profile snapshots are kept.", "Remove")) return;
        var ok = await AppState.RunAsync(() => AppState.Repo.RemovePersonAsync(id));
        if (!ok) return;
        _ = AppState.LoadPeopleAsync();
        AppState.Say("Removed from your list (profile snapshots kept).");
        if (Frame.CanGoBack) Frame.GoBack();
    }
}
