package ajustor.fonderie

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.material3.Badge
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.wrapContentSize
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import uniffi.fonderie.Tab
import uniffi.fonderie.TabInfo

private val BarBackground = Color(0xFF0B0B16)
private val Gold = Color(0xFFFFC850)
private val Muted = Color(0xFF9696AF)
private val Indicator = Color(0xFF1E1E32)
private val BadgeColor = Color(0xFFFF505A)

private fun label(tab: Tab) = when (tab) {
    Tab.FOUNDRY -> "Fonderie"
    Tab.FACTORIES -> "Usines"
    Tab.UPGRADE -> "Améliorer"
    Tab.COSMOS -> "Cosmos"
    Tab.MORE -> "Plus"
}

private fun icon(tab: Tab) = when (tab) {
    Tab.FOUNDRY -> "✦"
    Tab.FACTORIES -> "⚙"
    Tab.UPGRADE -> "▲"
    Tab.COSMOS -> "◎"
    Tab.MORE -> "≡"
}

/**
 * Barre d'onglets native : seuls les onglets débloqués apparaissent, une
 * pastille signale qu'une action attend.
 */
@Composable
fun TabBar(tabs: List<TabInfo>, font: FontFamily, onSelect: (Tab) -> Unit) {
    NavigationBar(containerColor = BarBackground) {
        for (info in tabs) {
            if (!info.unlocked) continue
            NavigationBarItem(
                selected = info.active,
                onClick = { onSelect(info.tab) },
                icon = {
                    BadgedBox(badge = { if (info.badge) Badge(containerColor = BadgeColor) }) {
                        // Boîte de taille fixe : les symboles des polices de
                        // repli sont plus hauts et décaleraient les libellés.
                        Box(Modifier.size(30.dp), contentAlignment = Alignment.Center) {
                            Text(
                                icon(info.tab),
                                fontFamily = font,
                                fontSize = 20.sp,
                                lineHeight = 30.sp,
                                maxLines = 1,
                                modifier = Modifier.wrapContentSize(unbounded = true),
                            )
                        }
                    }
                },
                label = {
                    Text(
                        label(info.tab),
                        fontFamily = font,
                        fontSize = 11.sp,
                        maxLines = 1,
                        softWrap = false,
                    )
                },
                colors = NavigationBarItemDefaults.colors(
                    selectedIconColor = Gold,
                    selectedTextColor = Gold,
                    unselectedIconColor = Muted,
                    unselectedTextColor = Muted,
                    indicatorColor = Indicator,
                ),
            )
        }
    }
}
