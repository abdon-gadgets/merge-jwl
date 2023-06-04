# JW Library Notes Sync

> **Warning**
> Because this web application lacks compliance with <https://www.jw.org/en/terms-use/>, it shuts down.
> Since June 2023, this GitHub repository no longer contains any source code.

JW-Sync is a utility to merge 2 or more `.jwlibrary` backup files,
containing your personal notes, highlighting, etc.
At time of writing, the JW Library app has backup and restore commands,
but no merge command.
With the official app, you can transfer user data between devices,
but you can't combine them into a single set.

You can use the web app <https://merge-jwl.netlify.app/> with Safari on iOS, Chrome on Android
and other modern browsers.
The `.jwlibrary` files that you upload stay on your device, they are not send over the internet.
This is possible because the app merges your data client side using WebAssembly.
Privacy Policy: The web server doesn't collect or process any user data.

This project is a port of <https://github.com/AntonyCorbett/JWLMerge>.
While JWLMerge is a desktop application (.NET Framework), this utility is programmed
using the Rust programming language to support more platforms:
Browser (WASM), Linux, MacOS and Windows.
