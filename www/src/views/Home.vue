<template>
  <div class="home">
    <h1>Merge JWL</h1>
    <p>
      Upload two or more
      <code>.jwlibrary</code> backup files to merge.
    </p>
    <MergeForm />
    <h2>Instructions</h2>
    <ol>
      <li>Open the JW Library app</li>
      <li>Go to the Personal Study page</li>
      <li>Click ‘Backup and Restore’</li>
      <li>Click ‘Create a backup’</li>
      <li>Save the file</li>
      <li>Run the same steps on your other devices that you want to sync</li>
      <li>Gather all backup files on one device (i.e. with cloud storage or email)</li>
      <li>
        Upload the files to this web app
        (<a href="https://merge-jwl.netlify.app/">merge-jwl.netlify.app</a>)
      </li>
      <li>Wait for this app to merge the files</li>
      <li>Download the resulting file and send it to all your devices</li>
      <li>Restore the backup in JW Library</li>
      <!-- <li>Click ‘Restore a backup’</li> -->
    </ol>
    <h2>Release Notes</h2>
    <ul>
      <li>
        <a href="https://github.com/abdon-gadgets/merge-jwl/releases/tag/v0.2">v0.2</a>
        wrote instructions and
        <a href="https://merge-jwl.netlify.app/about">about page</a>
      </li>
    </ul>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import MergeForm from "@/components/MergeForm.vue"; // @ is an alias to /src

export default defineComponent({
  name: "Home",
  components: {
    MergeForm,
  },
});
</script>
