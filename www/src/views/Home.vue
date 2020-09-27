<template>
  <div class="home">
    <MergeForm msg="JW Library Notes Sync"/>
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
