import { expect } from 'chai'
import { shallowMount } from '@vue/test-utils'
import MergeForm from '@/components/MergeForm.vue'

describe('MergeForm.vue', () => {
  it('renders props.msg when passed', () => {
    const msg = 'new message'
    const wrapper = shallowMount(MergeForm, {
      props: { msg }
    })
    expect(wrapper.text()).to.include(msg)
  })
})
