<template>
  <div class="hello">
    <h1>{{ msg }}</h1>
    <p>
      {{ wasmHello }}
    </p>
    <label>
        JW Library backup files
        <input type="file" required multiple accept=".jwlibrary" v-on:change="fileInputChange">
    </label>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import { startWasiTask, mergeUploads } from '../merge';

export default defineComponent({
  name: 'HelloWorld',
  props: {
    msg: String,
  },
  data() {
    return {
      wasmHello: "",
      // upload: null as FileList | null,
    }
  },
  methods: {
    fileInputChange: async function(e: Event) {
      const files = (e.target as HTMLInputElement).files;
      if (files && files.length > 1) {
        await mergeUploads(files);
      }
    },
  },
  async mounted() {
    await startWasiTask();
    this.wasmHello = "WebAssembly loaded";
  }
});
</script>

<!-- Add "scoped" attribute to limit CSS to this component only -->
<style scoped lang="scss">
h3 {
  margin: 40px 0 0;
}
ul {
  list-style-type: none;
  padding: 0;
}
li {
  display: inline-block;
  margin: 0 10px;
}
a {
  color: #42b983;
}
</style>
