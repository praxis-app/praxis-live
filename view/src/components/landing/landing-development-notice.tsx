import { FlaskConical } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const LandingDevelopmentNotice = () => {
  const { t } = useTranslation();

  return (
    <aside
      aria-labelledby="development-status-title"
      className="border-border my-8 flex gap-4 rounded-3xl border bg-neutral-50/80 px-6 py-6 sm:my-12 sm:px-8 dark:bg-white/3"
    >
      <div className="bg-blurple-1/10 text-blurple-2 dark:text-blurple-3 flex size-11 shrink-0 items-center justify-center rounded-2xl">
        <FlaskConical className="size-5" aria-hidden="true" />
      </div>
      <div>
        <h2
          id="development-status-title"
          className="text-lg font-semibold tracking-tight"
        >
          {t('landing.developmentStatus.title')}
        </h2>
        <p className="text-muted-foreground mt-1 leading-7">
          {t('landing.developmentStatus.description')}
        </p>
      </div>
    </aside>
  );
};
