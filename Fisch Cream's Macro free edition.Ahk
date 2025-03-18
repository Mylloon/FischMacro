#SingleInstance, force  ; Ensures only one instance of the script is running at a time
Coordmode, Mouse, Screen
CoordMode, Pixel, Screen
SetMouseDelay, -1 ; Disables mouse movement delay for faster execution
SetBatchLines, -1 ; Disables batch line delays for faster execution

Process, Priority,, H ; Makes it high priority

if (A_ScreenDPI * 100 // 96 != 100) {
    Run, ms-settings:display
    msgbox, 0x1030, WARNING!!, % "Your Display Scale seems to be a value other than 100`%. This means the macro will NOT work correctly!`n`nTo change this, right-click on your Desktop -> Click 'Display Settings' -> Under 'Scale & Layout', set Scale to 100`% -> Close and Restart Roblox before starting the macro.", 60
    ExitApp
}

message=
(
To use this macro, ensure you're in full-screen mode on Roblox and have Roblox Camera mode enabled.
)

; Display the message in a message box with a timeout of 60 seconds
msgbox, , Fisch Cream's Macro, % message, 60

; Activate the Roblox window for the macro to work with
WinActivate, Roblox ahk_exe RobloxPlayerBeta.exe
Sleep 1000

; If Roblox doesn't activate, show an error message
If !WinActive("Roblox ahk_exe RobloxPlayerBeta.exe") {
    Msgbox Failed to activate Roblox
    ExitApp
}
; = = = = = = = = = = = = = = = = = = = =
; Get the position and dimensions of the Roblox window
WinGetPos,,, RobloxWidth,RobloxHeight, Roblox ahk_exe RobloxPlayerBeta.exe
if (RobloxWidth < A_ScreenWidth && RobloxHeight < A_ScreenHeight) {
    Send {f11}
    Sleep 1000
}
WinGetPos,,, RobloxWidth,RobloxHeight, Roblox ahk_exe RobloxPlayerBeta.exe
; = = = = = = = = = = = = = = = = = = = =
ColorUI := {"0xffdcac": 10, "0x3d381b": 10, "0xfffde4": 10}
IngameUiXLeft := Round((RobloxWidth / 2560) * 2260)   ; Left X coordinate for ingame-ui
IngameUiYLeft := Round((RobloxHeight / 1080) * 980)   ; Left Y coordinate for ingame-ui
IngameUiXRight := Round((RobloxWidth / 2560) * 2560)  ; Right X coordinate for ingame-ui
IngameUiYRight := Round((RobloxHeight / 1080) * 1080) ; Right Y coordinate for ingame-ui
Loop, 5 {
    For Color, Variation in ColorUI {
        PixelSearch,,, %IngameUiXLeft%, %IngameUiYLeft%, %IngameUiXRight%, %IngameUiXRight%, %Color%, %Variation%, Fast RGB
        If (ErrorLevel = 0) {
            Sleep 1000
            X := Round((RobloxWidth / 2560) * 2525)
            If (RobloxWidth >= A_ScreenWidth) && (RobloxHeight >= A_ScreenHeight) {
                Click, %X%, 35
            } else {
                Click, %X%, 65
            }
            Sleep 1000
            Break
        }
    }
}

; = = = = = = = = = = = = = = = = = = = =
; Define coordinates for the mini-game area
Global MiniGameXLeft := Round((RobloxWidth / 2560) * 763)   ; Left X coordinate for mini-game
Global MiniGameYLeft := Round((RobloxHeight / 1080) * 899)  ; Left Y coordinate for mini-game
Global MiniGameXRight := Round((RobloxWidth / 2560) * 1796) ; Right X coordinate for mini-game
Global MiniGameYRight := Round((RobloxHeight / 1080) * 939) ; Right Y coordinate for mini-game

; = = = = = = = = = = = = = = = = = = = =
; Define coordinates for the shake area (used to detect screen shake)
Global ShakeXLeft  := Round((RobloxWidth / 800) * 100)   ; Left X coordinate for shake detection
Global ShakeYLeft  := Round((RobloxHeight / 800) * 175)  ; Left Y coordinate for shake detection
Global ShakeXRight := Round((RobloxWidth / 800) * 700)   ; Right X coordinate for shake detection
Global ShakeYRight := Round((RobloxHeight / 800) * 675)  ; Right Y coordinate for shake detection

; = = = = = = = = = = = = = = = = = = = =
; Define color targets for different screen elements (fish, white, and bar)
Global Color_Fish := {"0x434b5b": 3, "0x4a4a5c": 4, "0x47515d": 4}  ; Color for fish (with variation)
Global Color_White := {"0xFFFFFF": 15} ; Color for white (used for detecting certain screen regions)
Global Color_Bar := {"0x848587": 4, "0x787773": 4, "0x7a7873": 4} ; Colors for bar (with variations)

; = = = = = = = = = = = = = = = = = = = =
RunTime := A_TickCount ; Stores the script's start time.

Reels()
LastShakeTimer := A_TickCount
Loop {
    WinActivate, Roblox ahk_exe RobloxPlayerBeta.exe
    ; If a click shake is detected, update the last shake time
    If ClickShake(X, Y) {
        LastShakeTimer := A_TickCount
    }

    ; If 7 seconds have passed since the last shake, trigger the Reels function
    If (A_TickCount - LastShakeTimer > 7000) {
        Send m
        Sleep 250
        Reels()
    }

    ; If the fish and white color are found, proceed with fishing actions
    If (Search(Color_Fish) && Search(Color_White)) {
        If (!Control) {
            Loop, 50 {
                PixelSearch, BarX, , MiniGameXLeft, MiniGameYLeft, MiniGameXRight, MiniGameYRight, 0xFFFFFF, 15, Fast RGB
                if (ErrorLevel = 0) {
                    Control := (MiniGameXRight - (BarX - MiniGameXLeft)) - BarX
                    Break
                }
            }
            If (Control <= 0) {
                IniRead, Control, Settings.ini, Fisch, Control, 0
                IniWrite, %Control%, Settings.ini, Fisch, Control ; Autosave for Control
                Control := Round((RobloxWidth/800) * ((320 * Control) + 97))
            }
        }
        Loop {
            ; Get the current position of the fish
            FishPos := Search(Color_Fish)
            ; If no fish is found, exit the loop
            If (!FishPos) {
                Break
            }
            ; If fish is on the left side of the control bar, stop clicking (Mouse(0))
            if (FishPos < MiniGameXLeft + (Control * 0.8)) {
                Mouse(0)
                Continue
            }
            ; If fish is on the right side of the control bar, start clicking (Mouse(1))
            else if (FishPos > MiniGameXRight - (Control * 0.8)) {
                Mouse(1)
                Continue
            }

            ; Search for the white bar and adjust position, or search for the fallback bar color
            Bar := Search(Color_White) ? Search(Color_White) + Round(Control * 0.5) : Search(Color_Bar)
            ; If no bar is found, output a debug message and continue
            if !Bar {
                OutputDebug, Didn't Find Any Bar Color
                Continue
            }

            ; Calculate the range between the fish and the bar
            Range := FishPos - Bar

            ; If the fish is within the valid range, hold the mouse button
            If (Range >= 0) {
                Mouse(1)
                HoldTimer := A_TickCount
                OriginalPos := Bar
                Loop {
                    ; Continuously check the fish position and calculate the range
                    FishPos := Search(Color_Fish)
                    Range := FishPos - OriginalPos
                    Hold := HoldFormula(Range)

                    ; If holding conditions are met or fish position is invalid, exit the loop
                    if (!Hold || !FishPos || (A_TickCount - HoldTimer >= Hold) || Wait(10)) {
                        Success := (A_TickCount - HoldTimer >= Hold)
                        Break
                    }
                }

                ; If the action was successful, release the mouse and wait for the next action
                if (Success) {
                    Mouse(0)
                    Wait((A_TickCount - HoldTimer) * 0.6)
                }
            } else {
                ; If the fish is outside the range, release the mouse and adjust accordingly
                HoldTimer := A_TickCount
                Mouse(0)
                Range := Abs(Range)
                ContinueNow := False

                ; Wait for the required hold time and continue if conditions are met
                if (Wait(HoldFormula(Range) * 0.7)) {
                    Continue
                }
                Loop {
                    ; Search for the fish position
                    FishPos := Search(Color_Fish)
                    ; Break if the fish is not found or the white bar is detected
                    if (!FishPos || Search(Color_White)) {
                        Break
                    }

                    ; If conditions are met, continue the loop
                    if (Wait(10)) {
                        ContinueNow := True
                        Break
                    }

                    ; Adjust the current position of the bar and check if the fish is in range
                    CurrentPosition := Search(Color_Bar) - ((RobloxWidth / 800) * 30)
                    if (CurrentPosition <= FishPos) {
                        Break
                    }
                }

                ; If the action should continue, skip the rest of the code
                if (ContinueNow) {
                    Continue
                }

                ; Start holding the mouse and wait for the next action
                Mouse(1)
                if (Wait(A_TickCount - HoldTimer)) {
                    Mouse(0)
                    Continue
                }

                ; Release the mouse after the action
                Mouse(0)
            }
        }
        ; Wait for 1 second before repeating the loop and trigger the Reels action
        Sleep 1000
        Reels()

        ; Reset the last shake time
        LastShakeTimer := A_TickCount
    }
}
Return

; Wait function that waits for a specific condition based on the fish's position and time
Wait(Time) {
    SleepTimer := A_TickCount  ; Save the current time as SleepTimer
    Loop {
        ; Search for the fish position
        FishPos := Search(Color_Fish)

        ; If no fish is found or the fish is outside the valid range, return the result (True if found, False if not)
        if !FishPos || (FishPos < MiniGameXLeft || FishPos > MiniGameXRight)
            return !!FishPos

        ; If the elapsed time exceeds the given time, break the loop
        if (A_TickCount - SleepTimer > Time)
            break
    }
    return False  ; If the loop finishes without a break, return False
}

; Function to search for a color within a defined area and return the X position
Search(Target) {
    ; Loop through each color and variation pair in the target list
    for Color, Variation in Target {
        ; Perform pixel search to find the color in the specified area
        PixelSearch, XPosition,, MiniGameXLeft, MiniGameYLeft, MiniGameXRight, MiniGameYRight, Color, Variation, Fast RGB
        if !ErrorLevel  ; If the color is found (ErrorLevel is 0)
            return XPosition  ; Return the X position of the found color
    }
    return False  ; Return False if no matching color is found
}

; Function to simulate mouse button press (down or up) based on the state
Mouse(State) {
    ; Check if the current mouse button state (down/up) matches the requested state
    If (GetKeyState("LButton", "P") != State) {
        ; If not, click the mouse down or up depending on the requested state
        Click, % State ? "Down" : "Up"
    }
}

; Function to handle the Reels action (reeling the fishing line)
Reels() {
    global
    ; Move the mouse to a specific position
    MouseMove, 80, 400

    ; Send the 'm' key to indicate the reeling action
    Send m
    ; Hold the left mouse button down for 2 seconds to reel in
    Click, Down
    Sleep Random(600, 1200)
    Click, Up  ; Release the mouse button
    Sleep Random(1000, 1200)
}

; Function wrapping Random in a range
Random(Min, Max) {
    Random, i, Min, Max
    Return i
}

; Function to detect a "shake" action by checking if the pixel position has changed
ClickShake(ByRef ClickX, ByRef ClickY) {
    static MemoryX, MemoryY  ; Declare that variables that will be used here only
    ; Search for a white pixel within a specific area (shake detection)
    PixelSearch, ClickX, ClickY, %ShakeXLeft%, %ShakeYLeft%, %ShakeXRight%, %ShakeYRight%, 0xFFFFFF, 0, Fast
    If (ErrorLevel = 0) {  ; If the pixel is found
        ; Small correction to the X value
        ClickX += 25

        ; If the pixel position has changed, perform a click at the new position
        if (ClickX != MemoryX || ClickY != MemoryY) {
            Click, %ClickX%, %ClickY%
            MemoryX := ClickX, MemoryY := ClickY  ; Save the new position to memory
            Sleep 100
            Return True  ; Return True to indicate a successful shake detection
        }
    }
    Return False  ; Return False if no shake was detected
}

; Hotkey to exit the script when the spacebar is pressed
$space::exitapp

; Function to calculate hold time based on pixel distance (for Cream's Macro)
HoldFormula(pixel) {
    global RobloxWidth ; Use the global RobloxWidth variable
    ; Define a set of data pairs for calculating hold times
    data := [[0, 0], [16, 0], [132, 1], [217, 5], [365, 29], [450, 54], [534, 91], [632, 151], [736, 234], [817, 310], [900, 382], [997, 469], [1081, 541], [1164, 613], [1250, 686], [1347, 711], [1448, 721], [1531, 724], [1531, 9999]]

    ; Adjust data based on Roblox width (scaling based on screen width)
    for i, pair in data {
        pair[2] := Floor(pair[2] * (RobloxWidth / 800))  ; Scale the third value of each pair
        if (pixel < pair[2]) {
            lower := data[i - 1], upper := pair  ; Find the lower and upper pairs for interpolation
            break
        }
    }

    hold := lower[1] + (pixel - lower[2]) * (upper[1] - lower[1]) / (upper[2] - lower[2])
    ; Perform linear interpolation to calculate the hold time based on the pixel distance
    return % hold
}
