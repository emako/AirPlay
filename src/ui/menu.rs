use super::gdi::wide;
use super::Ctx;
use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, TrackPopupMenu, HMENU, MF_CHECKED, MF_GRAYED,
    MF_POPUP, MF_SEPARATOR, MF_STRING, TPM_LEFTALIGN, TPM_RIGHTBUTTON,
};

pub const ID_PLAY: usize = 2201;
pub const ID_STOP: usize = 2202;
pub const ID_PREV: usize = 2203;
pub const ID_NEXT: usize = 2204;
pub const ID_RANDOM: usize = 2205;
pub const ID_REPEAT: usize = 2206;
pub const ID_SINGLE: usize = 2207;
pub const ID_PL_LOOP: usize = 2208;
pub const ID_ADD_FILE: usize = 2210;
pub const ID_ADD_DIR: usize = 2211;
pub const ID_ADD_URL: usize = 2212;
pub const ID_NEW_LIST: usize = 2213;
pub const ID_DEL_LIST: usize = 2214;
pub const ID_DEL_TRACK: usize = 2215;
pub const ID_RECYCLE: usize = 2216;
pub const ID_CLEAR: usize = 2217;
pub const ID_RELOAD: usize = 2218;
pub const ID_LOCATE: usize = 2219;
pub const ID_FIND: usize = 2220;
pub const ID_SORT: usize = 2221;
pub const ID_GROUP: usize = 2222;
pub const ID_LRC: usize = 2230;
pub const ID_DESK: usize = 2231;
pub const ID_DESK_LOCK: usize = 2232;
pub const ID_COVER: usize = 2233;
pub const ID_VIS: usize = 2234;
pub const ID_EQ: usize = 2235;
pub const ID_MINI: usize = 2236;
pub const ID_TOP: usize = 2237;
pub const ID_FULL: usize = 2238;
pub const ID_MAX: usize = 2239;
pub const ID_SKIN: usize = 2240;
pub const ID_BLUR: usize = 2241;
pub const ID_SIDE: usize = 2242;
pub const ID_SETTINGS: usize = 2250;
pub const ID_ABOUT: usize = 2251;
pub const ID_EXIT: usize = 2252;
pub const ID_SHOW: usize = 2253;

pub fn popup(hwnd: HWND, ctx: &Ctx, x: i32, y: i32) {
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return };
        let app = super::lock(&ctx.app);
        let sort = app.lists.get(app.view).map(|l| l.sort).unwrap_or(false);
        let group = app.lists.get(app.view).map(|l| l.group).unwrap_or(false);
        item(menu, ID_PLAY, "播放 / 暂停", false, true);
        item(menu, ID_STOP, "停止", false, true);
        item(menu, ID_PREV, "上一首", false, true);
        item(menu, ID_NEXT, "下一首", false, true);
        sep(menu);
        item(menu, ID_RANDOM, "随机", app.random, true);
        item(menu, ID_SINGLE, "单曲循环", app.track_play, true);
        item(menu, ID_REPEAT, "列表循环", app.repeat_list, true);
        item(menu, ID_PL_LOOP, "列表切换", app.pl_loop, true);
        sep(menu);
        let Ok(list) = CreatePopupMenu() else { return };
        item(list, ID_ADD_FILE, "添加文件", false, true);
        item(list, ID_ADD_DIR, "添加文件夹", false, true);
        item(list, ID_ADD_URL, "添加网络地址", false, true);
        item(list, ID_NEW_LIST, "新建列表", false, true);
        item(list, ID_DEL_LIST, "删除列表", false, true);
        item(list, ID_DEL_TRACK, "从列表移除", false, true);
        item(list, ID_RECYCLE, "删除到回收站", false, true);
        item(list, ID_CLEAR, "清空列表", false, true);
        item(list, ID_RELOAD, "重新读取列表", false, true);
        item(list, ID_LOCATE, "定位文件", false, true);
        item(list, ID_FIND, "查找", false, true);
        item(list, ID_SORT, "排序", sort, true);
        item(list, ID_GROUP, "之后按目录分组", group, true);
        popup_item(menu, list, "列表");
        let Ok(view) = CreatePopupMenu() else { return };
        item(view, ID_LRC, "歌词", app.show_lrc, true);
        item(view, ID_DESK, "桌面歌词", app.desktop_lrc, true);
        item(view, ID_DESK_LOCK, "桌面歌词穿透", app.desk_lock, true);
        item(view, ID_COVER, "封面", app.show_cover, true);
        item(view, ID_VIS, "频谱", app.show_vis, true);
        item(view, ID_EQ, "均衡器", app.show_eq, true);
        item(view, ID_MINI, "迷你模式", app.mini, true);
        item(view, ID_TOP, "置顶", app.on_top, true);
        item(view, ID_MAX, "最大化", app.maximized, true);
        item(view, ID_FULL, "全屏", app.fullscreen, true);
        item(view, ID_SKIN, "换肤", false, true);
        item(view, ID_BLUR, "模糊 / 穿透感", app.transparent, true);
        item(view, ID_SIDE, "侧栏", app.show_sidebar, true);
        popup_item(menu, view, "界面");
        sep(menu);
        item(menu, ID_SETTINGS, "设置", false, true);
        item(menu, ID_SHOW, "显示主窗口", false, true);
        item(menu, ID_ABOUT, "关于", false, true);
        item(menu, ID_EXIT, "退出", false, true);
        drop(app);
        let _ = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON,
            x,
            y,
            0,
            hwnd,
            None,
        );
        let _ = DestroyMenu(menu);
    }
}

fn item(menu: HMENU, id: usize, text: &str, checked: bool, enabled: bool) {
    let mut flags = MF_STRING;
    if checked {
        flags |= MF_CHECKED;
    }
    if !enabled {
        flags |= MF_GRAYED;
    }
    let w = wide(text);
    unsafe {
        let _ = AppendMenuW(menu, flags, id, PCWSTR(w.as_ptr()));
    }
}

fn sep(menu: HMENU) {
    unsafe {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    }
}

fn popup_item(parent: HMENU, child: HMENU, text: &str) {
    let w = wide(text);
    unsafe {
        let _ = AppendMenuW(parent, MF_POPUP, child.0 as usize, PCWSTR(w.as_ptr()));
    }
}
