import { Github } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export const LandingFooter = () => {
  const { t } = useTranslation();

  return (
    <footer className="border-border border-t px-4 py-8 sm:px-6 lg:px-8">
      <div className="text-muted-foreground mx-auto flex max-w-6xl flex-col gap-4 text-sm sm:flex-row sm:items-center sm:justify-between">
        <div className="space-y-1">
          <p>{t('landing.footer.description')}</p>
          <p>{t('landing.footer.developmentStatus')}</p>
        </div>
        <a
          href="https://github.com/praxis-app/praxis"
          target="_blank"
          rel="noreferrer"
          className="text-foreground inline-flex w-fit items-center gap-2 font-medium hover:underline"
        >
          <Github className="size-4" aria-hidden="true" />
          {t('landing.footer.viewSource')}
        </a>
      </div>
    </footer>
  );
};
