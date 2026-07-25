import { LandingBenefitSection } from '@/components/landing/landing-benefit-section';
import { LandingDesktopHero } from '@/components/landing/landing-desktop-hero';
import { LandingDevelopmentNotice } from '@/components/landing/landing-development-notice';
import { LandingFooter } from '@/components/landing/landing-footer';
import { LandingHeader } from '@/components/landing/landing-header';
import { LandingPrimaryButton } from '@/components/landing/landing-primary-button';
import { LandingVisual } from '@/components/landing/landing-visual';
import { Button } from '@/components/ui/button';
import { NavigationPaths } from '@/constants/shared.constants';
import { useAuthData } from '@/hooks/use-auth-data';
import {
  ArrowDown,
  CheckCircle2,
  MessageCircle,
  MessagesSquare,
  Users,
  Vote,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';

export const PublicLandingPage = () => {
  const {
    inviteToken,
    isFirstUserLoading,
    isLoggedIn,
    isRegistered,
    showSignUp,
    signUpPath,
  } = useAuthData({ isFirstUserQueryEnabled: true });

  const { t } = useTranslation();

  const primaryCta = inviteToken
    ? t('landing.actions.acceptInvite')
    : t('landing.actions.signUp');

  const isSignUpLoading = !inviteToken && isFirstUserLoading;

  return (
    <div className="bg-background text-foreground min-h-dvh overflow-hidden">
      <LandingHeader
        canSignUp={showSignUp}
        isRegistered={isRegistered}
        isSignUpLoading={isSignUpLoading}
        primaryCta={primaryCta}
        showLogIn={!isLoggedIn}
        signUpPath={signUpPath}
      />

      <main>
        <section className="bg-blurple-1 relative overflow-hidden px-5 pt-10 pb-8 text-center text-white sm:px-8 lg:hidden">
          <div className="relative mx-auto max-w-xl">
            <div className="absolute -top-16 -right-20 size-56 rounded-full border border-white/20" />
            <div className="absolute top-36 -left-28 size-64 rounded-full bg-white/8" />
            <div className="absolute -right-10 bottom-28 size-36 rounded-full bg-indigo-950/12 blur-2xl" />

            <div className="relative">
              <h1 className="mx-auto max-w-md text-[2.6rem] leading-[0.98] font-bold tracking-[-0.045em] text-balance">
                {t('landing.hero.title')}
              </h1>
              <p className="mx-auto mt-5 max-w-md text-base leading-7 text-indigo-50">
                {t('landing.mobileHero.description')}
              </p>

              <div className="mx-auto mt-7 flex max-w-sm flex-col gap-3">
                <LandingPrimaryButton
                  canSignUp={showSignUp}
                  isRegistered={isRegistered}
                  size="lg"
                  className="h-13 w-full rounded-xl bg-white px-6 text-base text-indigo-950 shadow-lg shadow-indigo-950/15 hover:bg-indigo-50"
                  isLoading={isSignUpLoading}
                  label={primaryCta}
                  showArrow
                  signUpPath={signUpPath}
                />
                <Button
                  asChild
                  size="lg"
                  variant="outline"
                  className="h-13 w-full rounded-xl border-white/30 bg-white/10 px-6 text-base text-white hover:bg-white/15 hover:text-white"
                >
                  <Link to={NavigationPaths.Explore}>
                    {t('landing.actions.explore')}
                  </Link>
                </Button>
              </div>

              <div className="text-foreground relative mt-8 rounded-3xl border border-white/40 bg-white p-4 text-left shadow-2xl shadow-indigo-950/25">
                <div className="mb-4 flex items-center justify-between px-1">
                  <p className="text-sm font-bold">
                    {t('landing.mobileHero.visualTitle')}
                  </p>
                  <span className="text-muted-foreground flex items-center gap-1.5 text-xs">
                    <Users className="size-3.5" aria-hidden="true" />
                    {t('landing.mobileHero.online')}
                  </span>
                </div>

                <div className="rounded-2xl bg-neutral-100 p-4 dark:bg-neutral-900">
                  <div className="flex items-center gap-2 text-sm font-semibold">
                    <span className="text-blurple-2 flex size-8 items-center justify-center rounded-full bg-white shadow-sm dark:bg-neutral-800">
                      <MessageCircle className="size-4" aria-hidden="true" />
                    </span>
                    {t('landing.mobileHero.conversationTitle')}
                  </div>
                  <p className="text-muted-foreground mt-2 pl-10 text-xs leading-5">
                    {t('landing.mobileHero.conversationDescription')}
                  </p>
                </div>

                <div className="flex h-9 items-center justify-center">
                  <span className="bg-blurple-1 flex size-6 items-center justify-center rounded-full text-white shadow-sm">
                    <ArrowDown className="size-3.5" aria-hidden="true" />
                  </span>
                </div>

                <div className="bg-blurple-1/10 rounded-2xl p-4">
                  <div className="flex items-center gap-2 text-sm font-semibold">
                    <span className="text-blurple-2 flex size-8 items-center justify-center rounded-full bg-white shadow-sm dark:bg-neutral-800">
                      <CheckCircle2 className="size-4" aria-hidden="true" />
                    </span>
                    {t('landing.mobileHero.decisionTitle')}
                  </div>
                  <p className="text-muted-foreground mt-2 pl-10 text-xs leading-5">
                    {t('landing.mobileHero.decisionDescription')}
                  </p>
                </div>
              </div>
            </div>
          </div>

          <p className="mx-auto mt-6 max-w-68 text-center text-xs leading-5 font-medium text-balance text-indigo-100">
            {t('landing.hero.openSource')}
          </p>
        </section>

        <LandingDesktopHero
          canSignUp={showSignUp}
          isRegistered={isRegistered}
          isSignUpLoading={isSignUpLoading}
          primaryCta={primaryCta}
          signUpPath={signUpPath}
        />

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
            visual={<LandingVisual variant="conversation" />}
          />
          <LandingDevelopmentNotice />
          <LandingBenefitSection
            eyebrow={t('landing.forums.eyebrow')}
            title={t('landing.forums.title')}
            description={t('landing.forums.description')}
            icon={CheckCircle2}
            visual={<LandingVisual variant="forum" />}
            reversed
          />
          <LandingBenefitSection
            eyebrow={t('landing.decisions.eyebrow')}
            title={t('landing.decisions.title')}
            description={t('landing.decisions.description')}
            icon={Vote}
            visual={<LandingVisual variant="decision" />}
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
              <LandingPrimaryButton
                canSignUp={showSignUp}
                isRegistered={isRegistered}
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
