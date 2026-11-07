import '@fontsource/atkinson-hyperlegible-next/400.css'
import '@fontsource/atkinson-hyperlegible-next/600.css'
import '@fontsource/atkinson-hyperlegible-next/700.css'
import '@fontsource/atkinson-hyperlegible-mono/400.css'
import './styles/tokens.css'
import './styles/base.css'

import { createApp } from 'vue'

import { httpGateway } from './adapters/httpGateway'
import App from './App.vue'

createApp(App, { gateway: httpGateway() }).mount('#app')
