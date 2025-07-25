<template>
  <div class="update-view">
    <div class="view-header">
      <div class="header-content">
        <h1>{{ t('update.title') }}</h1>
        <p class="view-subtitle">{{ t('update.subtitle') }}</p>
      </div>
      <div class="header-actions">
        <button @click="goToSettings" class="settings-link">
          <Settings :size="16" />
          {{ t('navigation.settings') }}
        </button>
      </div>
    </div>

    <div class="update-content">
      <UpdateManagerCard :is-standalone="true" />

      <div class="update-info-section">
        <div class="info-card">
          <h3>{{ t('update.aboutUpdates') }}</h3>
          <ul class="info-list">
            <li>{{ t('update.autoCheckInfo') }}</li>
            <li>{{ t('update.securityInfo') }}</li>
            <li>{{ t('update.backupInfo') }}</li>
          </ul>
        </div>

        <div class="info-card">
          <h3>{{ t('update.releaseChannels') }}</h3>
          <p>{{ t('update.releaseChannelsInfo') }}</p>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'
import { useTranslation } from '../composables/useI18n'
import { Settings } from 'lucide-vue-next'
import UpdateManagerCard from '../components/dashboard/UpdateManagerCard.vue'

const { t } = useTranslation()
const router = useRouter()

const goToSettings = () => {
  router.push('/settings')
}
</script>

<style scoped>
.update-view {
  padding: 2rem;
  max-width: 1200px;
  margin: 0 auto;
  min-height: calc(100vh - 4rem);
}

.view-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 2rem;
  gap: 2rem;
}

.header-content {
  flex: 1;
}

.view-header h1 {
  font-size: 2.5rem;
  font-weight: 700;
  color: var(--color-text);
  margin: 0 0 0.5rem 0;
}

.view-subtitle {
  font-size: 1.125rem;
  color: var(--color-text-secondary);
  margin: 0;
  line-height: 1.6;
}

.header-actions {
  display: flex;
  gap: 1rem;
  align-items: center;
}

.settings-link {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  color: var(--color-text);
  text-decoration: none;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.9rem;
}

.settings-link:hover {
  background: var(--color-surface-hover);
  border-color: var(--color-primary);
  color: var(--color-primary);
}

.update-content {
  display: grid;
  grid-template-columns: 1fr;
  gap: 2rem;
}

.update-info-section {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 1.5rem;
  margin-top: 2rem;
}

.info-card {
  padding: 1.5rem;
  background: var(--color-surface);
  border: 1px solid var(--color-border);
  border-radius: 12px;
}

.info-card:hover {
  border-color: var(--color-primary-light);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.05);
}

.info-card h3 {
  margin: 0 0 1rem 0;
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--color-text);
}

.info-card p {
  margin: 0;
  color: var(--color-text-secondary);
  line-height: 1.6;
}

.info-list {
  margin: 0;
  padding-left: 1.5rem;
  color: var(--color-text-secondary);
}

.info-list li {
  margin-bottom: 0.5rem;
  line-height: 1.5;
}

.info-list li:last-child {
  margin-bottom: 0;
}

/* Dark theme adjustments */
:root.dark .info-card,
:root.auto.dark .info-card {
  background: var(--color-background-secondary);
  border-color: var(--color-border-dark);
}

:root.dark .info-card:hover,
:root.auto.dark .info-card:hover {
  border-color: var(--color-primary);
}

@media (max-width: 768px) {
  .update-view {
    padding: 1rem;
  }

  .view-header {
    flex-direction: column;
    align-items: stretch;
    gap: 1rem;
  }

  .view-header h1 {
    font-size: 2rem;
  }

  .view-subtitle {
    font-size: 1rem;
  }

  .header-actions {
    justify-content: flex-start;
  }

  .update-info-section {
    grid-template-columns: 1fr;
    gap: 1rem;
  }

  .info-card {
    padding: 1rem;
  }
}

@media (max-width: 480px) {
  .view-header h1 {
    font-size: 1.75rem;
  }
}
</style>
