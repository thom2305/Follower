// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

@file:OptIn(ExperimentalMaterial3Api::class)

package ca.thoms.follower.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import ca.thoms.follower.AppViewModel
import ca.thoms.follower.Route
import ca.thoms.follower.Tab

@Composable
fun FollowerApp(vm: AppViewModel) {
    val snackbar = remember { SnackbarHostState() }
    LaunchedEffect(Unit) { vm.messages.collect { snackbar.showSnackbar(it) } }

    val top = vm.backStack.lastOrNull()
    BackHandler(enabled = top != null) { vm.pop() }

    Scaffold(
        snackbarHost = { SnackbarHost(snackbar) },
        contentWindowInsets = WindowInsets(0, 0, 0, 0),
        bottomBar = {
            if (top == null) {
                NavigationBar {
                    Tab.entries.forEach { t ->
                        NavigationBarItem(
                            selected = vm.tab == t,
                            onClick = { vm.tab = t },
                            icon = { Icon(t.icon, contentDescription = null) },
                            label = { Text(t.label) },
                        )
                    }
                }
            }
        },
    ) { pad ->
        androidx.compose.foundation.layout.Box(Modifier.padding(pad)) {
            when (top) {
                is Route.PersonRoute -> PersonScreen(vm, top.userId)
                is Route.ArchiveRoute -> ArchiveDetailScreen(vm, top.msg)
                null -> when (vm.tab) {
                    Tab.People -> PeopleScreen(vm)
                    Tab.Lookup -> LookupScreen(vm)
                    Tab.Birthdays -> BirthdaysScreen(vm)
                    Tab.Archive -> ArchiveScreen(vm)
                    Tab.Settings -> SettingsScreen(vm)
                }
            }
        }
    }
}
