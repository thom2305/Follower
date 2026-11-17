// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Thom (thom2305)

package ca.thoms.follower

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.lifecycle.viewmodel.compose.viewModel
import ca.thoms.follower.ui.FollowerApp
import ca.thoms.follower.ui.theme.FollowerTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent {
            FollowerTheme {
                val vm: AppViewModel = viewModel()
                FollowerApp(vm)
            }
        }
    }
}
