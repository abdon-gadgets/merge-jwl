<template>
  <div class="hello">
    <h1>{{ msg }}</h1>
    <label>
      JW Library backup files
      <input
        type="file"
        required
        multiple
        accept=".jwlibrary"
        @change="fileInputChange"
      />
    </label>
    <p>{{ progress }}</p>
    <div>
      <!-- Errors -->
      <h3 v-if="errors[0]">Errors</h3>
      <ul>
        <li v-for="error in errors" :key="error">
          <ul class="message-error">
            <li v-for="line in error" :key="line">{{ line }}</li>
          </ul>
        </li>
      </ul>
      <!-- BookmarkOverflow -->
      <h3 v-if="bookmarks[0]">Discarded bookmarks (all 10 slots occupied)</h3>
      <ul>
        <li v-for="bookmark in bookmarks" :key="bookmark">
          <p>{{ bookmark.title }}</p>
          <small>{{ bookmark.snippet }}</small>
          <i>{{ bookmarkText(bookmark) }}</i>
        </li>
      </ul>
      <!-- NoteUpdate -->
      <h3 v-if="notes[0]">Notes that are updated</h3>
      <ul>
        <li v-for="note in notes" :key="note">
          <div v-for="text in [note.before, note.after]" :key="text">
            <p>{{ text.title }}</p>
            <small>{{ text.content }}</small>
            <i>{{ new Date(text.date).toLocaleString() }}</i>
          </div>
        </li>
      </ul>
      <button v-if="merge && merge.file" @click="download">Download merged</button>
    </div>
  </div>
</template>

<script lang="ts">
import { defineComponent } from "vue";
import {
  startWasi,
  mergeUploads,
  Progress,
  Merge,
  BookmarkOverflow,
  MessageJson,
  Note,
} from "../merge";

function updateProgress(progress: Progress) {
  console.debug("progress", progress);
  switch (progress) {
    case Progress.Load:
      return "Load";
    case Progress.Wasm:
      return "WASM";
    case Progress.Extract:
      return "Extract";
    case Progress.Merge:
      return "Merge";
    case Progress.Store:
      return "Store";
    case Progress.Pack:
      return "Pack";
    case Progress.Js:
      return "Finalize";
    default:
      return "Done";
  }
}

function mapMessage<T extends string | object>(
  merge: Merge | null,
  f: (m: MessageJson) => T | undefined
): T[] {
  return merge
    ? (merge.messages
        .map((m) => f(m))
        .filter((m) => typeof m !== "undefined") as T[])
    : [];
}

interface Data {
  progress: string;
  merge: Merge | null;
}

export default defineComponent({
  name: "HelloWorld",
  props: {
    msg: String,
  },
  data() {
    return {
      progress: "",
      merge: null,
    } as Data;
  },
  computed: {
    bookmarks(): BookmarkOverflow[] {
      return mapMessage(this.merge, (m) => m.bookmarkOverflow);
    },
    errors(): string[][] {
      return mapMessage(this.merge, (m) => m.error).map((e) =>
        e.split("\n").filter((m) => m.trim().length)
      );
    },
    notes(): Note[] {
      return mapMessage(this.merge, (m) => m.noteUpdate);
    },
  },
  methods: {
    fileInputChange: async function (e: Event) {
      const files = (e.target as HTMLInputElement).files;
      if (files && files.length > 1) {
        try {
          if (this.merge) {
            this.merge.drop();
          }
          this.merge = await mergeUploads(
            files,
            (p) => (this.progress = updateProgress(p))
          );
        } catch (e) {
          this.progress = e.toString();
        }
      }
    },
    download: function () {
      if (this.merge) {
        this.merge.download();
      }
    },
    bookmarkText: function (bookmark: BookmarkOverflow) {
      const issue = bookmark.issueTagNumber;
      if (issue > 1950_00 && issue < 2050_00) {
        return `${bookmark.keySymbol}.${issue % 100}`;
      }
      return bookmark.keySymbol;
    },
  },
  mounted() {
    startWasi();
  },
});
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss">
h3 {
  margin: 40px 0 0;
}
a {
  color: #42b983;
}

.message-error {
  background-color: #ffddcc;

  > li:first-child {
    font-weight: bold;
  }
}
</style>
