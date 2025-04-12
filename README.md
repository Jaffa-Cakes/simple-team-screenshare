# Simple Team Screenshare (STS)

You and your mates can now easily stream your games to each other using nothing more than [OBS](https://obsproject.com/) and a web browser. This is a simple solution to solve the problem of low quality Discord streams. It was made as a personal tool, so documentation is limited. The codebase is simple and easy to understand, so feel free to modify it to your liking. If you have any questions or suggestions, please open an issue on GitHub.

WARNING: This implementation is not encrypted or secure. Do not use this for ANYTHING sensitive. I don't currently have the time to implement a more secure version, but any contributions via Issues/Pull Requests are welcome. If you want to use this for anything sensitive, please implement your own security measures. This is a simple tool for streaming games to your friends, not a secure solution for anything else.

## Compiling/Building and Running

It is quite easy to build this project.

1. `npm run build` in `/frontend` to build the client.
2. `cargo build --release` in `/backend` to build the server. The built frontend is compiled into the final binary of the backend, so it can be run anywhere without needing to run the frontend separately.
3. You can now simply run the backend binary `/backend/target/release/backend`.

Access the TCP web interface at `http://localhost:7091/` and the UDP streaming server at `srt://localhost:7092`.

Port forward through your firewall and router to share with your friends so that they are able to stream to your server and access the web interface. Make sure you read the warning above before doing this. If you do not understand the implications of this, the tool likely is not for you in its current state.

# OBS Settings

Follow these settings to optimise your [OBS](https://obsproject.com/) for this streaming use case. Do not change any other settings unless you know what you are doing; changing them could use a lot more CPU and bandwidth or could cause your stream to be unstable. It won't be catastrophic, but it might take a bit of time to perfectly tune your settings for your system.

## File -> Settings -> Stream

- Server: `srt://localhost:7092?streamid=` insert your name in lowercase at the end of this link after the `=`. For example, `srt://localhost:7092?streamid=jedd`

## File -> Settings -> Output

- Output Mode: `Advanced`
- Video Encoder: `x264`
- Rescale Output: `Disabled` if your screen is 1080p, or `Bicubic` if your screen is larger than 1080p. You must set the rescale output resolution to `1920x1080`.
- Rate Control: `CBR`
- Bitrate: `8000 Kbps`. This should be lowered if there are a lot of people streaming at the same time or if you have a slow internet connection. You can lower it to `6000 Kbps` or even `4000 Kbps` if needed.
- Keyframe Interval: `2 s`
- CPU Usage Preset: `faster`
- Profile: `high`
- Tune: `zerolatency`

## File -> Settings -> Video

- Base (Canvas) Resolution: The resolution of your screen.
- Output (Scaled) Resolution: `1920x1080`
- Downscale Filter: `Bicubic`
- Common FPS Values: `60`

## Scenes/Sources/Audio Mixer

- Create a new scene for each individual game or program you want to stream, and an additional scene for your desktop so you can show your desktop when needed.
- For each game scene, add a new source and select `Game Capture`.
- For the desktop scene, add a new source and select `Display Capture`.
- Right-click on the preview and select `Lock Preview` to prevent accidental changes to the scene.
- Mute your desktop and microphone audio in OBS as it is assumed you are using Discord to talk with mates.

Click "Start Streaming" to begin your stream.
View all the streams from `http://localhost:7091/`. Running in an incognito window with no extensions is recommended for performance. Press F11 to put the website in fullscreen mode for a better experience.
