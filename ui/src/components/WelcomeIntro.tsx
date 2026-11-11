'use client';

import { useEffect, useRef, useState } from 'react';
import { motion, useReducedMotion } from 'motion/react';
import { ArrowRight, ArrowLeft, Monitor, Radio, SlidersHorizontal } from 'lucide-react';
import { Modal } from './Modal';
import { BrandMark } from './Brand';
import { motionTokens, shouldAnimate } from '../motion';
import './WelcomeIntro.css';

type Translate = (fr: string, en: string) => string;
const waveform = [.22, .4, .68, .38, .85, .56, .34, .72, 1, .64, .46, .8, .58, .3, .68, .94, .5, .36, .72, .46, .9, .6, .38, .7, .48, .28, .64, .88, .5, .32, .7, .4];

export function WelcomeIntro({ t, onComplete, onSetup }: { t: Translate; onComplete: () => void; onSetup?: () => void }) {
  const [step, setStep] = useState(0);
  const reduced = useReducedMotion();
  const animate = !reduced && shouldAnimate();
  const heading = useRef<HTMLHeadingElement>(null);
  const previousStep = useRef(step);
  useEffect(() => {
    if (previousStep.current !== step) { heading.current?.focus(); previousStep.current = step; }
  }, [step]);
  const slides = [
    { overline: t('01 / VOTRE SIGNAL', '01 / YOUR SIGNAL'), title: t('Votre stream.\nUn temps d’avance.', 'Your stream.\nTime on your side.'), detail: t('Gardez OBS, vos scènes et vos habitudes. Open-Delay se place entre votre logiciel et votre destination.', 'Keep OBS, your scenes and your workflow. Open-Delay sits between your software and your output.'), tag: t('Vos clés restent dans OBS', 'Your keys stay in OBS') },
    { overline: t('02 / VOTRE TEMPO', '02 / YOUR PACE'), title: t('Le bon recul.\nAu bon moment.', 'The right distance.\nAt the right moment.'), detail: t('Préparez votre historique, choisissez un délai et suivez ce qui est réellement envoyé à votre public.', 'Prepare your history, choose a delay and see what is actually being sent to your audience.'), tag: t('Délai effectif toujours visible', 'Effective delay always visible') },
    { overline: t('03 / VOTRE RÉGIE', '03 / YOUR STUDIO'), title: t('Tout sous les yeux.\nTout à portée de main.', 'Everything in view.\nEverything within reach.'), detail: t('Pilotez votre délai depuis la régie, un dock OBS ou Stream Deck. Personnalisez l’overlay pour informer votre public.', 'Control your delay from the studio, an OBS dock or Stream Deck. Customize the overlay to keep your audience informed.'), tag: t('Local. Gratuit. Sans compte.', 'Local. Free. No account.') },
  ];
  const slide = slides[step];
  const transition = { duration: animate ? motionTokens.duration.slow : motionTokens.duration.fast, ease: motionTokens.easing.smooth };

  return <Modal title={t('Bienvenue dans Open-Delay', 'Welcome to Open-Delay')} close={onComplete} footer={<>
        <div className="welcome-navigation">
          <ol aria-label={t('Présentation en trois étapes', 'Three-step introduction')}>{slides.map((item, index) => <li key={item.overline}><button type="button" aria-label={item.overline} aria-current={index === step ? 'step' : undefined} onClick={() => setStep(index)}><span/></button></li>)}</ol>
          <div className="welcome-actions">{step > 0 && <button className="button quiet" type="button" aria-label={t('Précédent', 'Back')} onClick={() => setStep(value => value - 1)}><ArrowLeft size={17}/></button>}<button type="button" className="button primary" onClick={() => step === 2 ? (onSetup ?? onComplete)() : setStep(value => value + 1)}>{step === 2 ? t('Configurer mon stream', 'Set up my stream') : t('Découvrir la suite', 'Keep exploring')}<ArrowRight size={17}/></button></div>
        </div>
        <button type="button" className="welcome-skip" onClick={onComplete}>{t('Passer la présentation', 'Skip introduction')}</button>
    </>}>
    <div className="welcome-intro">
      <div className="welcome-art" aria-hidden="true">
        <div className="welcome-art-grid"/>
        <div className="welcome-art-caption"><span>OPEN-DELAY</span><span>{t('PRÉSENTATION', 'INTRODUCTION')}</span></div>
        <motion.div className="welcome-emblem" initial={animate ? { scale: motionTokens.scale.entrance, opacity: 0 } : { opacity: 0 }} animate={animate ? { y: step === 1 ? -motionTokens.distance.sm : 0, scale: step === 2 ? motionTokens.scale.subtle : 1, opacity: 1 } : { opacity: 1 }} transition={transition}><BrandMark size={70}/></motion.div>
        <div className="welcome-signal-flow">
          <div className="welcome-signal-end"><Monitor size={17}/><span>OBS</span></div>
          <div className="welcome-waveform">{waveform.map((height, index) => <motion.i key={index} initial={animate ? { scaleY: motionTokens.scale.waveStart, opacity: 0 } : { opacity: 0 }} animate={animate ? { scaleY: step === 1 && index > 18 ? height * .3 : height, opacity: step === 1 && index > 18 ? .25 : .55 + height * .45 } : { opacity: .7 }} style={{ height: animate ? undefined : `${height * 100}%` }} transition={{ ...transition, delay: animate ? index * motionTokens.duration.instant / 3 : 0 }}/>)}</div>
          <div className="welcome-signal-end"><Radio size={17}/><span>{t('PUBLIC', 'AUDIENCE')}</span></div>
        </div>
        <motion.div className="welcome-demo-readout" initial={animate ? { y: motionTokens.distance.md, opacity: 0 } : { opacity: 0 }} animate={animate ? { y: step === 0 ? motionTokens.distance.sm : 0, opacity: step === 0 ? .55 : 1 } : { opacity: 1 }} transition={transition}>
          {step === 2 ? <><SlidersHorizontal size={18}/><span>{t('RÉGIE · OBS · STREAM DECK', 'STUDIO · OBS · STREAM DECK')}</span></> : <><span>{step === 1 ? t('EXEMPLE DE DÉLAI', 'EXAMPLE DELAY') : t('VOTRE CHAÎNE DE DIFFUSION', 'YOUR BROADCAST FLOW')}</span><strong>{step === 1 ? '30 s' : 'OBS → Open-Delay → Live'}</strong></>}
        </motion.div>
      </div>
      <div className="welcome-story">
        <p className="welcome-overline">{slide.overline}</p>
        <motion.div key={step} initial={animate ? { opacity: 0, y: motionTokens.distance.md } : { opacity: 0 }} animate={animate ? { opacity: 1, y: 0 } : { opacity: 1 }} transition={transition}>
          <h2 ref={heading} tabIndex={-1}>{slide.title}</h2>
          <p className="welcome-description">{slide.detail}</p>
          <p className="welcome-tag"><span/>{slide.tag}</p>
        </motion.div>

      </div>
    </div>
  </Modal>;
}
