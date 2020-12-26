<template>
  <div class="home">
    <h1>Merge JWL</h1>
    <p>
      Upload two or more <code>.jwlibrary</code> backup files.
    </p>
    <!-- TODO: Instructions for different operating systems -->
    <MergeForm/>
  </div>
</template>

<script lang="ts">
import { defineComponent } from 'vue';
import MergeForm from '@/components/MergeForm.vue'; // @ is an alias to /src

export default defineComponent({
  name: 'Home',
  components: {
    MergeForm,
  },
});
</script>
