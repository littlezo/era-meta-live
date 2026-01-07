import { createRouter, createWebHistory } from 'vue-router'
import PlayerView from '../pages/PlayerView.vue'
import SettingsView from '../pages/SettingsView.vue'
import HomeView from '../pages/HomeView.vue'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/settings',
      name: 'Settings',
      component: SettingsView
    },
    {
      path: '/home/:platform',
      name: 'HomePlatform',
      component: HomeView,
      props: true
    },
    {
      path: '/',
      redirect: '/home/douyin'
    },
    {
      path: '/player/:platform/:roomId',
      name: 'Player',
      component: PlayerView,
      props: true
    }
  ]
})

export default router