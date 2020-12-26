<template>
  <div class="about">
    <h1>About merge-jwl</h1>
    <p>
      At time of writing, the JW Library app has backup and restore commands,
      but no merge command.
      With the official app, you can transfer user data between devices,
      but you can't combine them into a single set.
      Merge JWL is a utility to merge 2 or more
      <code>.jwlibrary</code> backup files,
      containing your personal notes, highlighting, etc.
    </p>
    <h2>Contact</h2>
    <p>
      Mail <a href="mailto:abdon-gadgets@outlook.com">abdon-gadgets@outlook.com</a>
    </p>
    <h2>Privacy</h2>
    <p>
      The backup files you upload are not send over the internet,
      they stay on your device.
      This is possible because the app uses WebAssembly
      to merge the files client side.
    </p>
    <h2>Important Notes</h2>
    <p>
      Please use at your own risk!
      It is possible to crash JW Library by attempting to restore a corrupt backup file,
      and sometimes the only solution is to reinstall JW Library and reset its data.
    </p>
    <p>
      The authors of this software are not affiliated with the developers of JW Library,
      and the software should be considered unofficial.
      “JW Library” is a registered trademark of Watch Tower Bible and Tract Society of Pennsylvania.
      Please review the JW Library terms and conditions of use.
    </p>
    <h2>Source Code</h2>
    <p>
      Find the source code and licence at the
      <a
        href="https://github.com/abdon-gadgets/merge-jwl"
      >abdon-gadgets/merge-jwl</a>
      Github repository.
    </p>
  </div>
</template>
