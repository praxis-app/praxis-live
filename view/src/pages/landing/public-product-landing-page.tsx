import { LandingBenefitSection } from '@/components/landing/landing-benefit-section';
import { LandingFooter } from '@/components/landing/landing-footer';
import { LandingHeader } from '@/components/landing/landing-header';
import { LandingProductVisual } from '@/components/landing/landing-product-visual';
import { LandingSignUpButton } from '@/components/landing/landing-sign-up-button';
import { Button } from '@/components/ui/button';
import { NavigationPaths } from '@/constants/shared.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import { CheckCircle2, MessagesSquare, Vote } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

export const PublicProductLandingPage = () => {
  const { inviteToken, isFirstUserLoading, showSignUp, signUpPath } =
    useAuthData({ isFirstUserQueryEnabled: true });
  const { t } = useTranslation();

  const primaryCta = inviteToken
    ? t('landing.actions.acceptInvite')
    : t('landing.actions.signUp');
  const isSignUpLoading = !inviteToken && isFirstUserLoading;

  return (
    <div className="bg-background text-foreground min-h-dvh overflow-hidden">
      <LandingHeader
        canSignUp={showSignUp}
        isSignUpLoading={isSignUpLoading}
        primaryCta={primaryCta}
        signUpPath={signUpPath}
      />

      <main>
        <section className="relative px-4 pt-16 pb-24 sm:px-6 sm:pt-24 sm:pb-28 lg:px-8 lg:pt-28 lg:pb-36">
          <div className="bg-blurple-1/10 absolute top-8 left-1/2 z-0 size-128 -translate-x-1/2 rounded-full blur-3xl" />
          <div className="relative z-10 mx-auto grid max-w-6xl items-center gap-14 lg:grid-cols-[1fr_0.92fr] lg:gap-20">
            <div className="max-w-2xl">
              <p className="text-blurple-2 dark:text-blurple-3 mb-5 text-sm font-semibold tracking-[0.16em] uppercase">
                {t('landing.hero.eyebrow')}
              </p>
              <h1 className="text-5xl font-bold tracking-[-0.045em] text-balance sm:text-6xl lg:text-7xl">
                {t('landing.hero.title')}
              </h1>
              <p className="text-muted-foreground mt-6 max-w-xl text-lg leading-8 sm:text-xl">
                {t('landing.hero.description')}
              </p>
              <div className="mt-9 flex flex-col gap-3 sm:flex-row">
                <LandingSignUpButton
                  canSignUp={showSignUp}
                  size="lg"
                  className="bg-blurple-1 hover:bg-blurple-2 h-12 rounded-full px-7 text-base text-white"
                  isLoading={isSignUpLoading}
                  label={primaryCta}
                  showArrow
                  signUpPath={signUpPath}
                />
                <Button
                  asChild
                  size="lg"
                  variant="outline"
                  className="h-12 rounded-full px-7 text-base"
                >
                  <Link to={NavigationPaths.Explore}>
                    {t('landing.actions.explore')}
                  </Link>
                </Button>
              </div>
              <p className="text-muted-foreground mt-5 text-sm">
                {t('landing.hero.openSource')}
              </p>
            </div>

            <LandingProductVisual variant="flow" />
          </div>
        </section>

        <div className="border-border border-y bg-neutral-50/60 dark:bg-white/2">
          <div className="mx-auto max-w-6xl px-4 py-12 text-center sm:px-6 lg:px-8">
            <p className="text-2xl font-semibold tracking-tight text-balance sm:text-3xl">
              {t('landing.bridge')}
            </p>
          </div>
        </div>

        <div className="mx-auto max-w-6xl px-4 py-24 sm:px-6 sm:py-32 lg:px-8">
          <LandingBenefitSection
            eyebrow={t('landing.chat.eyebrow')}
            title={t('landing.chat.title')}
            description={t('landing.chat.description')}
            icon={MessagesSquare}
            visual={<LandingProductVisual variant="conversation" />}
          />
          <LandingBenefitSection
            eyebrow={t('landing.forums.eyebrow')}
            title={t('landing.forums.title')}
            description={t('landing.forums.description')}
            icon={CheckCircle2}
            visual={<LandingProductVisual variant="forum" />}
            reversed
          />
          <LandingBenefitSection
            eyebrow={t('landing.decisions.eyebrow')}
            title={t('landing.decisions.title')}
            description={t('landing.decisions.description')}
            icon={Vote}
            visual={<LandingProductVisual variant="decision" />}
          />
        </div>

        <section className="px-4 pb-24 sm:px-6 sm:pb-32 lg:px-8">
          <div className="bg-blurple-1 relative mx-auto max-w-6xl overflow-hidden rounded-4xl px-6 py-14 text-center text-white shadow-2xl shadow-indigo-950/20 sm:px-12 sm:py-20">
            <div className="absolute -top-24 -right-20 size-72 rounded-full border border-white/20" />
            <div className="absolute -bottom-28 -left-12 size-64 rounded-full bg-white/10" />
            <div className="relative mx-auto max-w-2xl">
              <h2 className="text-3xl font-bold tracking-tight text-balance sm:text-5xl">
                {t('landing.finalCta.title')}
              </h2>
              <p className="mt-5 text-lg leading-8 text-indigo-50">
                {t('landing.finalCta.description')}
              </p>
              <LandingSignUpButton
                canSignUp={showSignUp}
                size="lg"
                className="mt-8 h-12 rounded-full bg-white px-7 text-base text-indigo-950 hover:bg-indigo-50"
                isLoading={isSignUpLoading}
                label={primaryCta}
                showArrow
                signUpPath={signUpPath}
              />
            </div>
          </div>
        </section>
      </main>

      <LandingFooter />
    </div>
  );
};
