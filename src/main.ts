import { createApp } from 'vue'
import App from './App.vue'
import { getSunsetEffectEnabled, preloadSunsetEffect } from './services/completionAnimation'
import './styles/tokens.css'

if (getSunsetEffectEnabled()) void preloadSunsetEffect()

createApp(App).mount('#app')
