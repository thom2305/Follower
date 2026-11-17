// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

using System.Collections.ObjectModel;
using Follower.Data;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Controls.Primitives;

namespace Follower;

public sealed partial class BirthdaysPage : Page
{
    readonly ObservableCollection<Row> rows = new();
    bool ready;

    public BirthdaysPage()
    {
        InitializeComponent();
        NavigationCacheMode = Microsoft.UI.Xaml.Navigation.NavigationCacheMode.Required;
        List.ItemsSource = rows;
        ready = true;
        AppState.Changed += Render;
        Render();
    }

    void Render()
    {
        var days = (int)DaysSlider.Value;
        TitleText.Text = $"Next {days} days";
        var zone = AppState.Config.Zone;
        var upcoming = AppState.People
            .Where(e => e.Person.BirthdayMonth is not null && e.Person.BirthdayDay is not null)
            .Select(e => (Days: Util.DaysUntilBirthday(e.Person.BirthdayMonth!.Value, e.Person.BirthdayDay!.Value, zone), Entry: e))
            .Where(x => x.Days <= days)
            .OrderBy(x => x.Days).ToList();

        rows.Clear();
        foreach (var (_, e) in upcoming)
            rows.Add(new Row
            {
                Id = e.Person.UserId, Name = e.Name, Avatar = Ui.Img(e.Profile?.Avatar, 80),
                Subtitle = Util.FmtBirthday(e.Person.BirthdayMonth!.Value, e.Person.BirthdayDay!.Value, e.Person.BirthdayYear, zone),
            });
        EmptyText.Visibility = rows.Count == 0 ? Visibility.Visible : Visibility.Collapsed;
    }

    void DaysSlider_ValueChanged(object sender, RangeBaseValueChangedEventArgs e) { if (ready) Render(); }

    void List_ItemClick(object sender, ItemClickEventArgs e)
    {
        if (e.ClickedItem is Row r) Frame.Navigate(typeof(PersonPage), r.Id);
    }
}
