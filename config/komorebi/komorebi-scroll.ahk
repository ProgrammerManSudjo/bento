#Requires AutoHotkey v2.0
#SingleInstance Force
Persistent

; At login the main window can pop up if the script starts before the taskbar is ready: keep it hidden (tray icon is enough).
SetTimer(HideMain, -2500)
HideMain() {
    try WinHide("ahk_id " A_ScriptHwnd)     ; throws if it is already hidden, which is fine
}

; Apps that use modifier+scroll themselves (timeline/zoom). Hotkeys pass through here.
GroupAdd("Passthru", "ahk_exe Adobe Premiere Pro.exe")
GroupAdd("Passthru", "ahk_exe AfterFX.exe")
GroupAdd("Passthru", "ahk_exe BitwigStudioApp.exe")

; ---------------- Tray menu ----------------
tray := A_TrayMenu
tray.Delete()
tray.Add("Enabled", ToggleEnabled)
tray.Check("Enabled")
tray.Add()
tray.Default := "Enabled"
tray.ClickCount := 1          ; single left click toggles on/off
A_IconTip := "Komorebi scroll: ON"

ToggleEnabled(*) {
    Suspend(-1)               ; also swaps the tray icon to the suspended one
    if A_IsSuspended {
        tray.Uncheck("Enabled")
        A_IconTip := "Komorebi scroll: OFF"
    } else {
        tray.Check("Enabled")
        A_IconTip := "Komorebi scroll: ON"
    }
}

; ---------------- Game mode ----------------
; When Playnite starts a game (game-mode.ps1 on) everything that grabs keys or the mouse is switched off (no fullscreen auto-detection):
; these scroll hotkeys, komorebi with whkd (komorebic stop --whkd), YASB (yasbc stop) and AltSnap; the taskbar is set to auto-hide.
; When the game is gone it all comes back, but only what this script switched off itself.
global gameMode := false          ; game mode is on right now
global gameAuto := true           ; the tray's "Game mode (automatic)" switch
global gameHits := 0, calmHits := 0
global gamePid := 0               ; the game that switched game mode on: game mode ends when it is closed, not when you alt-tab out of it
global gsSuspended := false, gsAltSnap := "", gsKomorebi := false, gsYasb := false, gsTaskbar := false
; fullscreen programs that are not games (browsers, video players): they never switch game mode on
global notGames := ["chrome.exe", "msedge.exe", "firefox.exe", "zen.exe", "helium.exe", "vlc.exe", "mpv.exe", "Spotify.exe", "Plex.exe", "Netflix.exe"]

tray.Add()
tray.Add("Game mode now", (*) => (gameMode ? LeaveGameMode() : (gamePid := 0, EnterGameMode())))
tray.Add()
tray.Add("Exit (use before anticheat games)", (*) => ExitApp())
tray.Default := "Enabled"
tray.ClickCount := 1
SetTimer(GameWatch, 100)                  ; checking the flag file is cheap, so look often (it used to be every 2 s)
; game-mode.ps1 (Playnite's "before" and "after" game scripts) leaves "on" or "off" in this file; it is read on the next check
global gameFlag := A_AppData "\game-mode-flag.txt"

GameLog(text) {
    try FileAppend(FormatTime(, "yyyy-MM-dd HH:mm:ss") "  " text "`n", A_AppData "\game-mode.log")
}

GameWatch() {
    global gameMode, gameAuto, gameHits, calmHits, gamePid, gameFlag
    if FileExist(gameFlag) {
        cmd := ""
        try cmd := Trim(FileRead(gameFlag), " `t`r`n")
        try FileDelete(gameFlag)
        GameLog("flag file: '" cmd "' (game mode is " (gameMode ? "on" : "off") ")")
        if (cmd = "on" && !gameMode) {
            gamePid := 0                  ; Playnite says when the game ends: nothing to watch here
            EnterGameMode()
        } else if (cmd = "off" && gameMode) {
            LeaveGameMode()
        }
        return
    }
    if gameMode {
        ; on: it ends when the game itself is gone (alt-tabbing out of it does not count)
        if (gamePid != 0) {
            if ProcessExist(gamePid)
                calmHits := 0
            else
                calmHits++
            if (calmHits >= 20)                 ; gone for about 2 s in a row
                LeaveGameMode()
        }
        return
    }
    ; No fullscreen auto-detection any more: game mode only comes from Playnite (game-mode.ps1) or the tray's "Game mode now".
}

EnterGameMode() {
    global gameMode, gsSuspended, gsAltSnap, gsKomorebi, gsYasb, gsTaskbar
    gameMode := true
    GameLog("game mode ON")
    try {
        EnterSteps()
    } catch as e {
        GameLog("error while switching things off: " e.Message " (" e.What " line " e.Line ")")
    }
}

EnterSteps() {
    global gameMode, gsSuspended, gsAltSnap, gsKomorebi, gsYasb, gsTaskbar
    t0 := A_TickCount
    snap := MainMonitorWindows()                 ; first, before anything changes
    GameLog("  snapshot done +" (A_TickCount - t0) " ms")
    ; these scroll hotkeys
    gsSuspended := !A_IsSuspended
    if gsSuspended
        Suspend(1)
    ; AltSnap
    gsAltSnap := ""
    if (pid := ProcessExist("AltSnap.exe")) {
        gsAltSnap := A_AppData "\AltSnap\AltSnap.exe"
        try ProcessClose(pid)
        if ProcessExist("AltSnap.exe")
            RunWait("taskkill.exe /F /IM AltSnap.exe", , "Hide")
    }
    ; the taskbar: auto-hide on (instant, so it goes first)
    gsTaskbar := !(TaskbarState() & 1)
    if gsTaskbar {
        SetTaskbarAutoHide(true)
        GameLog("taskbar auto-hide: state " TaskbarState() " (bit 1 = on), taskbar window " WinExist("ahk_class Shell_TrayWnd"))
    }
    gsKomorebi := ProcessExist("komorebi.exe") ? true : false
    gsYasb := ProcessExist("yasb.exe") ? true : false
    if ProcessExist("whkd.exe")
        ProcessClose("whkd.exe")
    ; 1) YASB first: its bars reserve screen space, and komorebi has to see that space come back
    if gsYasb {
        while (pid := ProcessExist("yasb.exe")) {       ; ended directly: `yasbc stop` alone took ~2.5 s, and Windows frees the bar space when its window dies
            try ProcessClose(pid)
            ProcessWaitClose(pid, 2)
        }
        GameLog("  yasb ended +" (A_TickCount - t0) " ms")
        Loop 30 {                                       ; wait only until Windows has really given the space back (normally well under a second)
            if WorkAreasFree()
                break
            Sleep(50)
        }
    }
    GameLog("  yasb gone, space free: " (WorkAreasFree() ? "yes" : "NO") " +" (A_TickCount - t0) " ms")
    ; 2) komorebi re-tiles into the freed space, then lets go (stopping it restores the windows it had hidden); the windows stay tiled where they are
    if gsKomorebi {
        Sleep(100)                              ; komorebi's own work-area update
        RunWait("komorebic.exe retile", , "Hide")
        GameLog("  retile done +" (A_TickCount - t0) " ms")
        Sleep(150)                              ; let the new layout land before komorebi is stopped
        RunWait("komorebic.exe stop --whkd", , "Hide")
        GameLog("  komorebi stopped +" (A_TickCount - t0) " ms")
    }
    ; 3) everything that was on the main monitor is minimized
    n := MinimizeWindows(snap)
    GameLog("minimized " n " of " snap.Length " windows on the main monitor, all done +" (A_TickCount - t0) " ms")
}

; The windows on the main (primary) monitor right now. It is taken before game mode changes anything and before the game opens its own
; window, so the game (and anything that appears later) is never minimized. Skips the taskbar, desktop, tool windows, dialogs
; (they belong to an app window), minimized ones and windows Windows itself has cloaked (suspended apps, other virtual desktops).
MainMonitorWindows() {
    list := []
    try MonitorGet(MonitorGetPrimary(), &mL, &mT, &mR, &mB)
    catch
        return list
    skipClass := ["Shell_TrayWnd", "Shell_SecondaryTrayWnd", "Progman", "WorkerW", "XamlExplorerHostIslandWindow"]
    skipExe := ["yasb.exe", "komorebi-bar.exe", "AutoHotkey64.exe", "AutoHotkey32.exe", "TextInputHost.exe"]
    for hwnd in WinGetList() {
        try {
            id := "ahk_id " hwnd
            if !(WinGetStyle(id) & 0x10000000)          ; WS_VISIBLE
                continue
            if (WinGetExStyle(id) & 0x80)               ; WS_EX_TOOLWINDOW
                continue
            if DllCall("GetWindow", "Ptr", hwnd, "UInt", 4, "Ptr")   ; GW_OWNER
                continue
            if (WinGetMinMax(id) = -1 || WinGetTitle(id) = "")
                continue
            if HasValue(skipClass, WinGetClass(id)) || HasValue(skipExe, WinGetProcessName(id))
                continue
            cloaked := 0
            DllCall("dwmapi\DwmGetWindowAttribute", "Ptr", hwnd, "UInt", 14, "UInt*", &cloaked, "UInt", 4)
            if (cloaked & 6)                            ; cloaked by the shell / inherited (komorebi's own cloak is 1, so those stay in)
                continue
            WinGetPos(&x, &y, &w, &h, id)
            cx := x + w // 2, cy := y + h // 2
            if (cx >= mL && cx < mR && cy >= mT && cy < mB)
                list.Push(hwnd)
        }
    }
    return list
}

; True when every monitor's usable area equals the whole monitor (no bar or taskbar is reserving space any more)
WorkAreasFree() {
    Loop MonitorGetCount() {
        MonitorGet(A_Index, &l, &t, &r, &b)
        MonitorGetWorkArea(A_Index, &wl, &wt, &wr, &wb)
        if (wl != l || wt != t || wr != r || wb != b)
            return false
    }
    return true
}

; Temporary check: logs the monitor and where the windows on the non-primary monitor sit right after the re-tile
LogOtherMonitor() {
    prim := MonitorGetPrimary()
    Loop MonitorGetCount() {
        if (A_Index = prim)
            continue
        MonitorGet(A_Index, &l, &t, &r, &b)
        GameLog("  monitor " A_Index ": " l "," t " to " r "," b " (" (r - l) "x" (b - t) ")")
        for hwnd in WinGetList() {
            try {
                id := "ahk_id " hwnd
                if (!(WinGetStyle(id) & 0x10000000) || (WinGetExStyle(id) & 0x80) || WinGetMinMax(id) = -1 || WinGetTitle(id) = "")
                    continue
                if DllCall("GetWindow", "Ptr", hwnd, "UInt", 4, "Ptr")
                    continue
                cloaked := 0
                DllCall("dwmapi\DwmGetWindowAttribute", "Ptr", hwnd, "UInt", 14, "UInt*", &cloaked, "UInt", 4)
                if cloaked
                    continue
                WinGetPos(&x, &y, &w, &h, id)
                if (x + w // 2 >= l && x + w // 2 < r && y + h // 2 >= t && y + h // 2 < b)
                    GameLog("    " WinGetProcessName(id) ": " x "," y " " w "x" h)
            }
        }
    }
}

; Minimizes the windows from MainMonitorWindows() that are visible now (komorebi's stop has uncloaked them). Returns how many.
MinimizeWindows(list) {
    n := 0
    for hwnd in list {
        try {
            id := "ahk_id " hwnd
            if (!WinExist(id) || WinGetMinMax(id) = -1)
                continue
            cloaked := 0
            DllCall("dwmapi\DwmGetWindowAttribute", "Ptr", hwnd, "UInt", 14, "UInt*", &cloaked, "UInt", 4)
            if cloaked
                continue
            DllCall("ShowWindowAsync", "Ptr", hwnd, "Int", 6)   ; SW_MINIMIZE, without waiting for a window that is not answering
            n++
        }
    }
    return n
}

HasValue(arr, val) {
    for v in arr
        if (v = val)
            return true
    return false
}

LeaveGameMode() {
    global gamePid, gameMode
    gameMode := false
    gamePid := 0
    GameLog("game mode OFF")
    try {
        LeaveSteps()
    } catch as e {
        GameLog("error while switching things back on: " e.Message " (" e.What " line " e.Line ")")
    }
}

LeaveSteps() {
    global gsSuspended, gsAltSnap, gsKomorebi, gsYasb, gsTaskbar
    if gsTaskbar
        SetTaskbarAutoHide(false)
    ; everything starts together; none of it waits for komorebi to answer any more
    if gsKomorebi
        Run("komorebic.exe start --whkd", , "Hide")
    if gsYasb
        Run("yasb.exe", , "Hide")                  ; straight to the exe: about 0.3 s faster than going through `yasbc start --silent`
    if (gsAltSnap != "" && !ProcessExist("AltSnap.exe"))
        Run(gsAltSnap, A_AppData "\AltSnap")
    if gsSuspended
        Suspend(0)
    gsSuspended := false, gsAltSnap := "", gsKomorebi := false, gsYasb := false, gsTaskbar := false
}

; Taskbar auto-hide through the shell's own app bar message (the same as the taskbar setting)
TaskbarState() {
    abd := Buffer(48, 0)
    NumPut("UInt", 48, abd, 0)
    NumPut("Ptr", WinExist("ahk_class Shell_TrayWnd"), abd, 8)
    return DllCall("shell32\SHAppBarMessage", "UInt", 4, "Ptr", abd, "UPtr")   ; ABM_GETSTATE: bit 1 = auto-hide, bit 2 = always on top
}

SetTaskbarAutoHide(on) {
    state := TaskbarState()
    abd := Buffer(48, 0)
    NumPut("UInt", 48, abd, 0)
    NumPut("Ptr", WinExist("ahk_class Shell_TrayWnd"), abd, 8)
    NumPut("Int64", on ? (state | 1) : (state & ~1), abd, 40)
    DllCall("shell32\SHAppBarMessage", "UInt", 10, "Ptr", abd)                 ; ABM_SETSTATE
}


; ---------------- komorebic runner ----------------
; Throttle so fast scrolling doesn't queue up dozens of komorebic processes
lastTick := 0
Km(cmd) {
    global lastTick
    if (A_TickCount - lastTick < 80)
        return
    lastTick := A_TickCount
    Run("komorebic.exe " cmd, , "Hide")
}

; ---------------- Scroll bindings ----------------
#HotIf !WinActive("ahk_group Passthru")

; Scroll direction: Win groups (focus/move): wheel up = next, wheel down = previous
; Alt groups (workspaces): wheel up = previous, wheel down = next

; Win + scroll: cycle focus through windows on the current workspace (wraps)
#WheelUp::Km("cycle-focus next")
#WheelDown::Km("cycle-focus previous")

; Win + Shift + scroll: cycle focus between monitors
#+WheelUp::Km("cycle-monitor next")
#+WheelDown::Km("cycle-monitor previous")

; Win + Ctrl + scroll: cycle-move window through the workspace order
#^WheelUp::Km("cycle-move next")
#^WheelDown::Km("cycle-move previous")

; Win + Ctrl + Shift + scroll: move window to previous / next monitor
#^+WheelUp::Km("cycle-move-to-monitor next")
#^+WheelDown::Km("cycle-move-to-monitor previous")

; Alt + scroll: previous / next workspace
!WheelUp::Km("cycle-workspace previous")
!WheelDown::Km("cycle-workspace next")

; Win + Alt + scroll: send window to previous / next workspace
#!WheelUp::Km("cycle-move-to-workspace previous")
#!WheelDown::Km("cycle-move-to-workspace next")

#HotIf

; ---------------- Keyboard binds that whkd can't claim ----------------
; Win + Z / Win + Shift + Z: cycle focus between monitors (overrides the snap layouts flyout)
#z::Km("cycle-monitor next")
#+z::Km("cycle-monitor previous")

; Ctrl + Win + Z / Ctrl + Win + Shift + Z: move focused window to next / previous monitor
#^z::Km("cycle-move-to-monitor next")
#^+z::Km("cycle-move-to-monitor previous")
