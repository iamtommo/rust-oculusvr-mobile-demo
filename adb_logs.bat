: reasonably useful tags
: AndroidRuntime for crashes, VrApi/OVR for oculus, RustStdoutStderr for rust output
: Distortion for ovr
: DEBUG for crashes
: adb logcat -s "AndroidRuntime","VrApi","OVR","RustStdoutStderr","DEBUG","libEGL","libc","VrCubeWorld"
adb logcat AndroidRuntime:I VrApi:V OVR:V RustStdoutStderr:V DEBUG:I libEGL:I libc:I VrCubeWorld:V *:S


: adb tcpip 5555
: adb shell ip addr show wlan0
: adb connect <addr>