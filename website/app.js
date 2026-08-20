/**
 * ============================================================
 * RAVEN NOTCH — app.js
 * Complete interactive JavaScript for the Raven Notch website
 * ============================================================
 */

'use strict';


/* ============================================================
   LOADING SCREEN
   ============================================================ */
(function initLoadingScreen() {
  const screen = document.getElementById('loadingScreen');
  const bar = document.getElementById('loadingProgressBar');
  if (!screen) return;

  // Lock scroll while loading
  document.body.style.overflow = 'hidden';

  const startProgress = () => {
    const duration = 1800; // 1.8s loading animation
    const startTime = performance.now();

    function update(now) {
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);
      
      // macOS-style variable loading speed (starts fast, slows down in the middle, finishes quick)
      let displayProgress;
      if (progress < 0.4) {
        displayProgress = progress * 1.25; // fast start
      } else if (progress < 0.75) {
        displayProgress = 0.50 + (progress - 0.4) * 0.35; // slow crawl in middle
      } else {
        displayProgress = 0.6225 + (progress - 0.75) * 1.51; // final surge to 100%
      }
      displayProgress = Math.min(displayProgress, 1);

      if (bar) {
        bar.style.width = (displayProgress * 100) + '%';
      }

      if (progress < 1) {
        requestAnimationFrame(update);
      } else {
        setTimeout(() => {
          screen.classList.add('hidden');
          document.body.style.overflow = '';
        }, 350); // slight pause at 100% for impact, then fade out
      }
    }

    requestAnimationFrame(update);
  };

  if (document.readyState === 'loading') {
    window.addEventListener('DOMContentLoaded', startProgress);
  } else {
    startProgress();
  }
})();


/* ============================================================
   NAVIGATION — Frosted glass on scroll + Hamburger
   ============================================================ */
(function initNav() {
  const nav = document.getElementById('nav');
  const hamburger = document.getElementById('navHamburger');
  const mobileMenu = document.getElementById('navMobileMenu');

  if (!nav) return;

  // Scroll handler
  let ticking = false;
  function onScroll() {
    if (!ticking) {
      requestAnimationFrame(() => {
        if (window.scrollY > 20) {
          nav.classList.add('scrolled');
        } else {
          nav.classList.remove('scrolled');
        }
        ticking = false;
      });
      ticking = true;
    }
  }
  window.addEventListener('scroll', onScroll, { passive: true });

  // Hamburger toggle
  if (hamburger && mobileMenu) {
    hamburger.addEventListener('click', () => {
      hamburger.classList.toggle('open');
      mobileMenu.classList.toggle('open');
    });

    // Close mobile menu when link is clicked
    mobileMenu.querySelectorAll('a').forEach(link => {
      link.addEventListener('click', () => {
        hamburger.classList.remove('open');
        mobileMenu.classList.remove('open');
      });
    });
  }
})();


/* ============================================================
   SCROLL REVEAL — IntersectionObserver
   ============================================================ */
(function initScrollReveal() {
  const elements = document.querySelectorAll('.reveal');
  if (!elements.length) return;

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const el = entry.target;
        const delay = parseInt(el.dataset.delay) || 0;
        setTimeout(() => {
          el.classList.add('visible');
        }, delay);
        observer.unobserve(el);
      }
    });
  }, {
    threshold: 0.12,
    rootMargin: '0px 0px -40px 0px'
  });

  elements.forEach(el => observer.observe(el));
})();


/* ============================================================
   NOTCH PILL CYCLING ANIMATION
   Cycles: compact → music → stats → clock → repeat (every 2.8s)
   ============================================================ */
(function initPillAnimation() {
  const states = ['stateCompact', 'stateMusic', 'stateStats', 'stateClock'];
  let current = 0;

  function activateState(index) {
    states.forEach((id, i) => {
      const el = document.getElementById(id);
      if (!el) return;
      if (i === index) {
        el.classList.add('active');
      } else {
        el.classList.remove('active');
      }
    });
  }

  // Start with compact
  activateState(0);

  setInterval(() => {
    current = (current + 1) % states.length;
    activateState(current);
  }, 2800);
})();


/* ============================================================
   LIVE CLOCK — Updates every second
   ============================================================ */
(function initLiveClock() {
  function pad(n) {
    return String(n).padStart(2, '0');
  }

  const days = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];
  const months = ['January', 'February', 'March', 'April', 'May', 'June',
                  'July', 'August', 'September', 'October', 'November', 'December'];
  const shortMonths = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun',
                       'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const shortDays = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

  function updateClock() {
    const now = new Date();
    const h = pad(now.getHours());
    const m = pad(now.getMinutes());
    const timeStr = `${h}:${m}`;

    const dayName = days[now.getDay()];
    const shortDay = shortDays[now.getDay()];
    const month = months[now.getMonth()];
    const shortMonth = shortMonths[now.getMonth()];
    const date = now.getDate();
    const year = now.getFullYear();

    // Hero clock (bento clock card)
    const heroClock = document.getElementById('heroClock');
    if (heroClock) heroClock.textContent = timeStr;

    const heroDate = document.getElementById('heroDate');
    if (heroDate) heroDate.textContent = `${dayName}, ${month} ${date}`;

    // Pill clock state
    const pillClockTime = document.getElementById('pillClockTime');
    if (pillClockTime) pillClockTime.textContent = timeStr;

    const pillClockDate = document.getElementById('pillClockDate');
    if (pillClockDate) pillClockDate.textContent = `${shortDay}, ${shortMonth} ${date}`;

    // CTA columns clock
    const ctaClockL = document.getElementById('ctaClockL');
    if (ctaClockL) ctaClockL.textContent = timeStr;

    const ctaClockR = document.getElementById('ctaClockR');
    if (ctaClockR) ctaClockR.textContent = timeStr;
  }

  updateClock();
  setInterval(updateClock, 1000);
})();


/* ============================================================
   MINI CALENDAR — Fills the cal grid
   ============================================================ */
(function initCalendar() {
  const calGrid = document.getElementById('calGrid');
  const calMonth = document.getElementById('calMonth');
  if (!calGrid || !calMonth) return;

  const now = new Date();
  const year = now.getFullYear();
  const month = now.getMonth();
  const today = now.getDate();

  const months = ['January', 'February', 'March', 'April', 'May', 'June',
                  'July', 'August', 'September', 'October', 'November', 'December'];

  calMonth.textContent = `${months[month]} ${year}`;

  const dayHeaders = ['S', 'M', 'T', 'W', 'T', 'F', 'S'];
  dayHeaders.forEach(d => {
    const el = document.createElement('div');
    el.className = 'cal-day-header';
    el.textContent = d;
    calGrid.appendChild(el);
  });

  const firstDay = new Date(year, month, 1).getDay();
  const daysInMonth = new Date(year, month + 1, 0).getDate();

  // Empty cells before first day
  for (let i = 0; i < firstDay; i++) {
    const el = document.createElement('div');
    el.className = 'cal-day empty';
    calGrid.appendChild(el);
  }

  // Day cells
  for (let d = 1; d <= daysInMonth; d++) {
    const el = document.createElement('div');
    el.className = 'cal-day' + (d === today ? ' today' : '');
    el.textContent = d;
    calGrid.appendChild(el);
  }
})();


/* ============================================================
   SYSMONITOR — Simulated live data animation
   ============================================================ */
(function initSysmonitor() {
  const values = {
    cpu: { fill: 'cpuFill', pct: 'cpuPct', base: 28, variance: 18 },
    ram: { fill: 'ramFill', pct: 'ramPct', base: 58, variance: 8 },
    gpu: { fill: 'gpuFill', pct: 'gpuPct', base: 20, variance: 14 },
    net: { fill: 'netFill', pct: 'netPct', base: 40, variance: 22 }
  };

  let sparklineData = [50, 40, 45, 25, 35, 20, 30];

  function lerp(a, b, t) {
    return a + (b - a) * t;
  }

  function clamp(v, min, max) {
    return Math.min(max, Math.max(min, v));
  }

  let currentValues = {};
  Object.keys(values).forEach(k => {
    currentValues[k] = values[k].base;
  });

  let targetValues = {};
  function randomizeTargets() {
    Object.keys(values).forEach(k => {
      const cfg = values[k];
      targetValues[k] = clamp(cfg.base + (Math.random() - 0.5) * cfg.variance * 2, 3, 95);
    });
  }
  randomizeTargets();

  function updateSparkline() {
    sparklineData.shift();
    sparklineData.push(clamp(currentValues.cpu, 5, 90));

    const svgWidth = 180;
    const svgHeight = 60;
    const points = sparklineData.map((v, i) => {
      const x = (i / (sparklineData.length - 1)) * svgWidth;
      const y = svgHeight - (v / 100) * (svgHeight - 8) - 4;
      return `${x},${y}`;
    }).join(' ');

    const fillPoints = `0,${svgHeight} ` + points + ` ${svgWidth},${svgHeight}`;

    const lineEl = document.getElementById('sparklinePoints');
    const fillEl = document.getElementById('sparklineFill');
    if (lineEl) lineEl.setAttribute('points', points);
    if (fillEl) fillEl.setAttribute('points', fillPoints);
  }

  let frame = 0;
  function tick() {
    frame++;

    // Every 60 frames (~3s), randomize targets
    if (frame % 60 === 0) {
      randomizeTargets();
    }

    Object.keys(values).forEach(k => {
      const cfg = values[k];
      currentValues[k] = lerp(currentValues[k], targetValues[k], 0.04);

      const fillEl = document.getElementById(cfg.fill);
      const pctEl = document.getElementById(cfg.pct);

      if (fillEl) fillEl.style.width = currentValues[k].toFixed(1) + '%';
      if (pctEl) pctEl.textContent = Math.round(currentValues[k]) + '%';
    });

    if (frame % 12 === 0) {
      updateSparkline();
    }

    requestAnimationFrame(tick);
  }

  // Start after short delay for smooth appearance
  setTimeout(tick, 500);
})();


/* ============================================================
   KPI COUNTER ANIMATION — Counts up when in view
   ============================================================ */
(function initKPICounters() {
  const kpiCards = document.querySelectorAll('.kpi-card');
  if (!kpiCards.length) return;

  const downloadCountEl = document.getElementById('kpiDownloadsCount');

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const card = entry.target;
        const countEl = card.querySelector('[data-animate]');
        if (countEl) {
          countEl.dataset.animated = 'true';
          const target = parseInt(countEl.getAttribute('data-animate')) || 0;
          animateCount(countEl, 0, target, 1200);
        }
        observer.unobserve(card);
      }
    });
  }, { threshold: 0.4 });

  kpiCards.forEach(card => observer.observe(card));

  function animateCount(el, start, end, duration) {
    const startTime = performance.now();
    function frame(now) {
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);
      const eased = 1 - Math.pow(1 - progress, 3); // ease-out-cubic
      const current = Math.round(start + (end - start) * eased);
      el.textContent = current;
      if (progress < 1) {
        requestAnimationFrame(frame);
      }
    }
    requestAnimationFrame(frame);
  }

})();


/* ============================================================
   TESTIMONIALS CAROUSEL — Drag + prev/next + dots
   ============================================================ */
(function initTestimonialsCarousel() {
  const carousel = document.getElementById('testimonialsCarousel');
  const prevBtn = document.getElementById('carouselPrev');
  const nextBtn = document.getElementById('carouselNext');
  const dotsContainer = document.getElementById('carouselDots');

  if (!carousel) return;

  let cards = carousel.querySelectorAll('.testimonial-card');
  let cardCount = cards.length;
  let currentIndex = 0;

  function rebuildDots() {
    if (!dotsContainer) return;
    dotsContainer.innerHTML = '';
    cards.forEach((_, i) => {
      const dot = document.createElement('button');
      dot.className = 'carousel-dot' + (i === 0 ? ' active' : '');
      dot.setAttribute('aria-label', `Go to testimonial ${i + 1}`);
      dot.addEventListener('click', () => goTo(i));
      dotsContainer.appendChild(dot);
    });
  }

  // Expose function globally to update carousel when feedback is submitted
  window.refreshTestimonialsCarousel = function() {
    cards = carousel.querySelectorAll('.testimonial-card');
    cardCount = cards.length;
    rebuildDots();
    goTo(0); // Snap to start to show the newly added feedback!
  };

  // Build dots initially
  rebuildDots();

  function updateDots(index) {
    if (!dotsContainer) return;
    const dots = dotsContainer.querySelectorAll('.carousel-dot');
    dots.forEach((dot, i) => {
      dot.classList.toggle('active', i === index);
    });
  }

  function getCardWidth() {
    if (cards.length === 0) return 0;
    const rect = cards[0].getBoundingClientRect();
    const gap = 20;
    return rect.width + gap;
  }

  function goTo(index) {
    currentIndex = Math.max(0, Math.min(index, cardCount - 1));
    const offset = currentIndex * getCardWidth();
    carousel.scrollTo({ left: offset, behavior: 'smooth' });
    updateDots(currentIndex);
  }

  if (prevBtn) {
    prevBtn.addEventListener('click', () => {
      goTo(currentIndex - 1);
    });
  }

  if (nextBtn) {
    nextBtn.addEventListener('click', () => {
      goTo(currentIndex + 1);
    });
  }

  // Drag to scroll
  let isDragging = false;
  let startX = 0;
  let scrollStart = 0;

  carousel.addEventListener('mousedown', (e) => {
    isDragging = true;
    startX = e.clientX;
    scrollStart = carousel.scrollLeft;
    carousel.classList.add('dragging');
  });

  window.addEventListener('mousemove', (e) => {
    if (!isDragging) return;
    const dx = e.clientX - startX;
    carousel.scrollLeft = scrollStart - dx;
  });

  window.addEventListener('mouseup', () => {
    if (!isDragging) return;
    isDragging = false;
    carousel.classList.remove('dragging');

    // Snap to nearest card
    const cardW = getCardWidth();
    const nearest = Math.round(carousel.scrollLeft / cardW);
    currentIndex = Math.max(0, Math.min(nearest, cardCount - 1));
    updateDots(currentIndex);
  });

  // Touch support
  let touchStartX = 0;
  let touchScrollStart = 0;

  carousel.addEventListener('touchstart', (e) => {
    touchStartX = e.touches[0].clientX;
    touchScrollStart = carousel.scrollLeft;
  }, { passive: true });

  carousel.addEventListener('touchmove', (e) => {
    const dx = e.touches[0].clientX - touchStartX;
    carousel.scrollLeft = touchScrollStart - dx;
  }, { passive: true });

  carousel.addEventListener('touchend', () => {
    const cardW = getCardWidth();
    const nearest = Math.round(carousel.scrollLeft / cardW);
    currentIndex = Math.max(0, Math.min(nearest, cardCount - 1));
    goTo(currentIndex);
  });

  // Auto-advance every 5s
  let autoInterval = setInterval(() => {
    if (cardCount === 0) return;
    const next = (currentIndex + 1) % cardCount;
    goTo(next);
  }, 5000);

  // Pause on hover
  carousel.addEventListener('mouseenter', () => clearInterval(autoInterval));
  carousel.addEventListener('mouseleave', () => {
    autoInterval = setInterval(() => {
      if (cardCount === 0) return;
      const next = (currentIndex + 1) % cardCount;
      goTo(next);
    }, 5000);
  });
})();


/* ============================================================
   DOWNLOAD BUTTON TOAST & OVERLAY
   ============================================================ */
(function initDownloadButtons() {
  const toast = document.getElementById('toast');
  const toastText = document.getElementById('toastText');
  const downloadOverlay = document.getElementById('downloadOverlay');

  function showToast(message) {
    if (!toast) return;
    if (toastText) toastText.textContent = message;
    toast.classList.add('show');
    setTimeout(() => {
      toast.classList.remove('show');
    }, 3000);
  }

  window.showToast = showToast;

  const downloadBtns = [
    document.getElementById('downloadBtnHero'),
    document.getElementById('downloadBtnCta'),
    document.getElementById('navCta'),
    document.getElementById('downloadBtnMobile')
  ];

  // Set up download overlay handlers
  if (downloadOverlay) {
    downloadOverlay.addEventListener('click', () => {
      downloadOverlay.classList.remove('show');
    });
  }

  downloadBtns.forEach(btn => {
    if (!btn) return;
    btn.addEventListener('click', (e) => {
      // Increment client-side download counter dynamically
      const downloadCountEl = document.getElementById('kpiDownloadsCount');
      console.log('Download clicked! downloadCountEl:', downloadCountEl);
      if (downloadCountEl) {
        const currentTarget = parseInt(downloadCountEl.getAttribute('data-animate')) || 0;
        const newTarget = currentTarget + 1;
        console.log('Incrementing target from', currentTarget, 'to', newTarget, 'Animated status:', downloadCountEl.dataset.animated);
        downloadCountEl.setAttribute('data-animate', newTarget);

        const currentVal = parseInt(downloadCountEl.textContent) || 0;
        if (downloadCountEl.dataset.animated === 'true') {
          // Smooth count-up animation
          const startTime = performance.now();
          const duration = 600;
          function frame(now) {
            const elapsed = now - startTime;
            const progress = Math.min(elapsed / duration, 1);
            const eased = 1 - Math.pow(1 - progress, 3); // ease-out-cubic
            downloadCountEl.textContent = Math.round(currentVal + (newTarget - currentVal) * eased);
            if (progress < 1) {
              requestAnimationFrame(frame);
            }
          }
          requestAnimationFrame(frame);
        } else {
          // If not animated yet, set text directly so it counts up from it when intersected
          downloadCountEl.textContent = newTarget;
        }
      }

      if (downloadOverlay) {
        downloadOverlay.classList.add('show');
        // Auto hide overlay after 4 seconds
        setTimeout(() => {
          downloadOverlay.classList.remove('show');
        }, 4000);
      } else {
        showToast('📥 Starting download: Raven-Notch.exe...');
      }
    });
  });

  // Also catch any btn-primary with href="#"
  document.querySelectorAll('.btn-primary[href="#"]').forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      showToast('🚀 Coming Soon! Follow @ui_raunak for launch updates.');
    });
  });

  const licenseModal = document.getElementById('licenseResultModal');
  const licenseKeyBox = document.getElementById('licenseKeyBox');
  const licenseClose = document.getElementById('licenseResultClose');
  const copyLicenseBtn = document.getElementById('copyLicenseBtn');
  const emailModal = document.getElementById('emailCaptureModal');
  const emailForm = document.getElementById('emailCaptureForm');
  const emailInput = document.getElementById('purchaseEmailInput');
  const emailClose = document.getElementById('emailCaptureClose');
  const paymentVerifyModal = document.getElementById('paymentVerifyModal');
  let latestLicenseKey = '';
  let pendingEmailResolve = null;

  function showLicenseModal(licenseKey) {
    latestLicenseKey = licenseKey;
    try {
      localStorage.setItem('ravenLastLicenseKey', licenseKey);
    } catch {}
    if (licenseKeyBox) licenseKeyBox.textContent = licenseKey;
    if (licenseModal) {
      licenseModal.classList.add('show');
      licenseModal.setAttribute('aria-hidden', 'false');
    }
  }

  function revealLicenseKey(licenseKey) {
    showLicenseModal(licenseKey);
  }

  function showPaymentVerify() {
    if (!paymentVerifyModal) return;
    paymentVerifyModal.classList.add('show');
    paymentVerifyModal.setAttribute('aria-hidden', 'false');
  }

  function delay(ms) {
    return new Promise((resolve) => window.setTimeout(resolve, ms));
  }

  function waitForPaint() {
    return new Promise((resolve) => {
      window.requestAnimationFrame(() => window.requestAnimationFrame(resolve));
    });
  }

  function hidePaymentVerify() {
    if (!paymentVerifyModal) return;
    paymentVerifyModal.classList.remove('show');
    paymentVerifyModal.setAttribute('aria-hidden', 'true');
  }

  function closeLicenseModal() {
    if (!licenseModal) return;
    licenseModal.classList.remove('show');
    licenseModal.setAttribute('aria-hidden', 'true');
  }
  window.closeLicenseModal = closeLicenseModal;

  if (licenseClose) licenseClose.addEventListener('click', closeLicenseModal);
  const licenseResultCloseBtn = document.getElementById('licenseResultCloseBtn');
  if (licenseResultCloseBtn) {
    licenseResultCloseBtn.addEventListener('click', closeLicenseModal);
  }
  if (licenseModal) {
    licenseModal.addEventListener('click', (event) => {
      if (event.target === licenseModal) closeLicenseModal();
    });
  }
  if (copyLicenseBtn) {
    copyLicenseBtn.addEventListener('click', async () => {
      if (!latestLicenseKey) return;
      try {
        await navigator.clipboard.writeText(latestLicenseKey);
        showToast('License key copied.');
      } catch {
        showToast('Select and copy the license key manually.');
      }
    });
  }

  function closeEmailModal(value = null) {
    if (emailModal) {
      emailModal.classList.remove('show');
      emailModal.setAttribute('aria-hidden', 'true');
    }
    if (pendingEmailResolve) {
      pendingEmailResolve(value);
      pendingEmailResolve = null;
    }
  }

  function askForPurchaseEmail() {
    if (!emailModal || !emailForm || !emailInput) {
      return Promise.resolve('');
    }

    emailInput.value = '';
    emailModal.classList.add('show');
    emailModal.setAttribute('aria-hidden', 'false');

    window.setTimeout(() => emailInput.focus(), 80);
    return new Promise((resolve) => {
      pendingEmailResolve = resolve;
    });
  }

  if (emailClose) emailClose.addEventListener('click', () => closeEmailModal(null));
  if (emailModal) {
    emailModal.addEventListener('click', (event) => {
      if (event.target === emailModal) closeEmailModal(null);
    });
  }
  if (emailForm) {
    emailForm.addEventListener('submit', (event) => {
      event.preventDefault();
      const email = (emailInput?.value || '').trim();
      if (!email) {
        showToast('Enter your email to continue.');
        return;
      }
      closeEmailModal(email);
    });
  }

  async function getPurchaseAccount() {
    try {
      const response = await fetch('/api/auth/me', { credentials: 'include', cache: 'no-store' });
      const account = await response.json();
      return account?.authenticated ? account : null;
    } catch {
      return null;
    }
  }

  async function beginPurchase() {
    if (window.location.protocol === 'file:') {
      showToast('Payment needs Cloudflare Functions. Test from your live site or wrangler pages dev.');
      return;
    }

    if (!window.Razorpay) {
      showToast('Payment checkout is still loading. Try again in a moment.');
      return;
    }

    const account = await getPurchaseAccount();
    if (!account?.user?.email) {
      showToast('Log in first so this purchase can attach to your Raven account.');
      document.getElementById('navLogin')?.click();
      return;
    }

    const email = account.user.email;
    if (email === null) return;

    try {
      const orderResponse = await fetch('/api/create-order', {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ email })
      });
      const order = await orderResponse.json();
      if (!orderResponse.ok) throw new Error(order.error || 'Unable to create order');
      try {
        localStorage.setItem('ravenLastOrder', JSON.stringify({
          orderId: order.orderId,
          email,
          createdAt: new Date().toISOString()
        }));
      } catch {}

      const checkout = new window.Razorpay({
        key: order.keyId,
        amount: order.amount,
        currency: order.currency,
        name: order.name,
        description: order.description,
        order_id: order.orderId,
        prefill: { email },
        theme: { color: '#111111' },
        handler: async function(payment) {
          try {
            showPaymentVerify();
            await waitForPaint();
            const verifyStartedAt = window.performance.now();
            try {
              localStorage.setItem('ravenLastPayment', JSON.stringify({
                ...payment,
                email,
                verifiedAt: new Date().toISOString()
              }));
            } catch {}
            const verifyResponse = await fetch('/api/verify-payment', {
              method: 'POST',
              credentials: 'include',
              headers: { 'content-type': 'application/json' },
              body: JSON.stringify({ ...payment, email })
            });
            const result = await verifyResponse.json();
            if (!verifyResponse.ok) throw new Error(result.error || 'Payment verification failed');
            if (!result.licenseKey) throw new Error('Payment verified but license key was missing');
            await delay(Math.max(1400 - (window.performance.now() - verifyStartedAt), 0));
            hidePaymentVerify();
            const refreshedAccount = await getPurchaseAccount();
            if (refreshedAccount && window.applyAccountState) window.applyAccountState(refreshedAccount);
            revealLicenseKey(result.licenseKey);
          } catch (error) {
            hidePaymentVerify();
            showToast(error.message || 'Payment done, but license verification failed. Contact support with your payment id.');
          }
        }
      });
      checkout.on('payment.failed', function(response) {
        showToast(response?.error?.description || 'Payment failed. No money was captured.');
      });
      checkout.open();
    } catch (error) {
      showToast(error.message || 'Could not start purchase.');
    }
  }

  [document.getElementById('purchaseBtnHero'), document.getElementById('purchaseBtnCta')]
    .forEach((purchaseBtn) => {
      if (!purchaseBtn) return;
      purchaseBtn.addEventListener('click', (e) => {
        e.preventDefault();
        beginPurchase();
      });
    });
})();


/* ============================================================
   BUTTON HOVER — Ensure sliding text works for dynamically-
   created buttons (polyfill for older browsers)
   ============================================================ */
(function initButtonHovers() {
  // The CSS :hover handles the sliding animation.
  // This just ensures all .btn-primary elements have the correct
  // inner structure if any are dynamically added.
  function ensureSlideStructure(btn) {
    if (btn.querySelector('.btn-text-wrap')) return;
    const text = btn.textContent.trim();
    btn.innerHTML = `<span class="btn-text-wrap">
      <span class="btn-text">${text}</span>
      <span class="btn-text">${text}</span>
    </span>`;
  }

  document.querySelectorAll('.btn-primary:not([data-slide-init])').forEach(btn => {
    btn.dataset.slideInit = '1';
    if (!btn.querySelector('.btn-text-wrap')) {
      ensureSlideStructure(btn);
    }
  });
})();


/* ============================================================
   MARQUEE — Ensure smooth loop (JS backup)
   ============================================================ */
(function initMarquee() {
  const track = document.querySelector('.marquee-track');
  if (!track) return;
  // CSS animation handles it; JS just ensures no gap on resize.
  // Pause on hover for accessibility
  track.addEventListener('mouseenter', () => {
    track.style.animationPlayState = 'paused';
  });
  track.addEventListener('mouseleave', () => {
    track.style.animationPlayState = 'running';
  });
})();


/* ============================================================
   BENTO GRID — Staggered entrance animation
   ============================================================ */
(function initBentoEntrance() {
  const bentGrid = document.getElementById('bentoGrid');
  if (!bentGrid) return;

  const observer = new IntersectionObserver((entries) => {
    if (entries[0].isIntersecting) {
      const cards = bentGrid.querySelectorAll('.bento-card');
      cards.forEach((card, i) => {
        card.style.opacity = '0';
        card.style.transform = 'translateY(24px)';
        card.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        setTimeout(() => {
          card.style.opacity = '1';
          card.style.transform = 'translateY(0)';
        }, 100 + i * 80);
      });
      observer.unobserve(bentGrid);
    }
  }, { threshold: 0.1 });

  // Pre-hide
  bentGrid.querySelectorAll('.bento-card').forEach(card => {
    card.style.opacity = '0';
  });

  observer.observe(bentGrid);
})();


/* ============================================================
   SMOOTH SCROLL — For anchor links
   ============================================================ */
(function initSmoothScroll() {
  document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', (e) => {
      const href = anchor.getAttribute('href');
      if (href === '#') return; // handled by download buttons
      const target = document.querySelector(href);
      if (target) {
        e.preventDefault();
        const navH = parseInt(getComputedStyle(document.documentElement)
          .getPropertyValue('--nav-h')) || 72;
        const top = target.getBoundingClientRect().top + window.scrollY - navH - 20;
        window.scrollTo({ top, behavior: 'smooth' });
      }
    });
  });
})();


/* ============================================================
   FEATURE ITEMS — Stagger on scroll
   ============================================================ */
(function initFeatureItemReveal() {
  const featureItems = document.querySelectorAll('.feature-grid-card');
  if (!featureItems.length) return;

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const el = entry.target;
        const delay = parseInt(el.dataset.delay) || 0;
        setTimeout(() => {
          el.classList.add('visible');
        }, delay);
        observer.unobserve(el);
      }
    });
  }, { threshold: 0.15 });

  featureItems.forEach(el => observer.observe(el));
})();


/* ============================================================
   PARALLAX LIGHT RAYS — Subtle movement on mouse move
   ============================================================ */
(function initLightRaysParallax() {
  const raysContainer = document.querySelector('.rays-container');
  if (!raysContainer) return;

  let targetX = 0;
  let currentX = 0;

  document.addEventListener('mousemove', (e) => {
    const centerX = window.innerWidth / 2;
    targetX = (e.clientX - centerX) / centerX * 20;
  });

  function animateRays() {
    currentX += (targetX - currentX) * 0.05;
    raysContainer.style.transform = `translateX(calc(-50% + ${currentX}px))`;
    requestAnimationFrame(animateRays);
  }
  animateRays();
})();


/* ============================================================
   KPI CARDS — Dot grid pulse on hover & Spotlight Mouse Tracker
   ============================================================ */
(function initKpiHover() {
  document.querySelectorAll('.kpi-card, .bento-card, .testimonial-card, .feature-grid-card').forEach(card => {
    card.addEventListener('mouseenter', () => {
      const dotGrid = card.querySelector('.card-dot-grid');
      if (dotGrid) {
        dotGrid.style.backgroundImage =
          'radial-gradient(rgba(255,255,255,0.08) 1px, transparent 1px)';
      }
    });
    card.addEventListener('mouseleave', () => {
      const dotGrid = card.querySelector('.card-dot-grid');
      if (dotGrid) {
        dotGrid.style.backgroundImage =
          'radial-gradient(rgba(255,255,255,0.035) 1px, transparent 1px)';
      }
      card.style.setProperty('--mouse-x', '-9999px');
      card.style.setProperty('--mouse-y', '-9999px');
    });
    card.addEventListener('mousemove', (e) => {
      const rect = card.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      card.style.setProperty('--mouse-x', `${x}px`);
      card.style.setProperty('--mouse-y', `${y}px`);
    });
  });
})();


/* ============================================================
   PERFORMANCE — reduce animations when user prefers reduced motion
   ============================================================ */
(function initReducedMotion() {
  const mq = window.matchMedia('(prefers-reduced-motion: reduce)');
  if (mq.matches) {
    document.documentElement.style.setProperty('--ease-spring', 'ease');
    // Pause marquee
    const marqueeTrack = document.querySelector('.marquee-track');
    if (marqueeTrack) marqueeTrack.style.animationPlayState = 'paused';
    // Stop CTA scroll tracks
    document.querySelectorAll('.cta-scroll-up, .cta-scroll-down').forEach(el => {
      el.style.animationPlayState = 'paused';
    });
  }
})();


/* ============================================================
   DARK MOTION PARTICLES BACKGROUND
   ============================================================ */
(function initCanvasParticles() {
  const canvas = document.getElementById('bgCanvas');
  if (!canvas) return;

  const ctx = canvas.getContext('2d');
  let particles = [];
  // Adjust particle count based on screen area to ensure smooth performance
  const particleCount = Math.min(60, Math.floor((window.innerWidth * window.innerHeight) / 25000));
  const connectionDistance = 110;
  const mouse = { x: null, y: null, radius: 120 };

  // Set sizing
  function resizeCanvas() {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
  }
  resizeCanvas();
  window.addEventListener('resize', resizeCanvas, { passive: true });

  // Track mouse
  window.addEventListener('mousemove', (e) => {
    mouse.x = e.clientX;
    mouse.y = e.clientY;
  }, { passive: true });

  window.addEventListener('mouseleave', () => {
    mouse.x = null;
    mouse.y = null;
  });

  // Particle Class
  class Particle {
    constructor() {
      this.x = Math.random() * canvas.width;
      this.y = Math.random() * canvas.height;
      this.vx = (Math.random() - 0.5) * 0.25;
      this.vy = (Math.random() - 0.5) * 0.25;
      this.radius = Math.random() * 1.5 + 0.8;
      this.alpha = Math.random() * 0.15 + 0.05;
      this.baseAlpha = this.alpha;
    }

    update() {
      // Move particle
      this.x += this.vx;
      this.y += this.vy;

      // Wrap around screen bounds
      if (this.x < 0) this.x = canvas.width;
      if (this.x > canvas.width) this.x = 0;
      if (this.y < 0) this.y = canvas.height;
      if (this.y > canvas.height) this.y = 0;

      // Mouse interactivity (gentle repulsion)
      if (mouse.x !== null && mouse.y !== null) {
        const dx = this.x - mouse.x;
        const dy = this.y - mouse.y;
        const dist = Math.hypot(dx, dy);
        if (dist < mouse.radius) {
          const force = (mouse.radius - dist) / mouse.radius;
          const angle = Math.atan2(dy, dx);
          // Apply repulsion force pushing particle away from mouse
          this.x += Math.cos(angle) * force * 0.6;
          this.y += Math.sin(angle) * force * 0.6;
          this.alpha = Math.min(0.3, this.baseAlpha + force * 0.15);
        } else {
          this.alpha = this.baseAlpha;
        }
      } else {
        this.alpha = this.baseAlpha;
      }
    }

    draw() {
      ctx.beginPath();
      ctx.arc(this.x, this.y, this.radius, 0, Math.PI * 2);
      ctx.fillStyle = `rgba(255, 255, 255, ${this.alpha})`;
      ctx.fill();
    }
  }

  // Initialize particles list
  function initParticles() {
    particles = [];
    for (let i = 0; i < particleCount; i++) {
      particles.push(new Particle());
    }
  }
  initParticles();

  // Draw connections between close particles
  function drawConnections() {
    for (let i = 0; i < particles.length; i++) {
      for (let j = i + 1; j < particles.length; j++) {
        const p1 = particles[i];
        const p2 = particles[j];
        const dx = p1.x - p2.x;
        const dy = p1.y - p2.y;
        const dist = Math.hypot(dx, dy);

        if (dist < connectionDistance) {
          const alpha = (1 - dist / connectionDistance) * 0.07;
          ctx.beginPath();
          ctx.moveTo(p1.x, p1.y);
          ctx.lineTo(p2.x, p2.y);
          ctx.strokeStyle = `rgba(255, 255, 255, ${alpha})`;
          ctx.lineWidth = 0.8;
          ctx.stroke();
        }
      }
    }
  }

  // Animation loop
  function animate() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    particles.forEach(p => {
      p.update();
      p.draw();
    });

    drawConnections();

    requestAnimationFrame(animate);
  }
  animate();
})();

/* ============================================================
   APP SIMULATOR ENGINE — Reference Notch Driver
   ============================================================ */
(function initNotchEngine() {
  const notch = document.getElementById('ravenNotch');
  if (!notch) return;

  // Notch Interactive Guide Prompt
  const guidePrompt = document.getElementById('notchGuidePrompt');
  if (guidePrompt) {
    const hideGuide = () => {
      guidePrompt.classList.add('hidden');
      notch.removeEventListener('mouseenter', hideGuide);
      notch.removeEventListener('touchstart', hideGuide);
    };
    notch.addEventListener('mouseenter', hideGuide);
    notch.addEventListener('touchstart', hideGuide, { passive: true });
  }

  /* ── Module tab switching ── */
  const tabs = notch.querySelectorAll('.ne-tab');
  const panels = notch.querySelectorAll('.ne-module-panel');
  const neBody = notch.querySelector('.ne-body');

  const MODS = ['home', 'media', 'clock', 'shelf', 'stats'];

  tabs.forEach(tab => {
    tab.addEventListener('click', e => {
      e.stopPropagation();
      const newMode = tab.dataset.mod;
      const oldMode = neBody ? (neBody.getAttribute('data-active-mod') || 'home') : 'home';

      if (newMode === oldMode) return;

      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      if (neBody) {
        // Calculate transition direction
        const oldIndex = MODS.indexOf(oldMode);
        const newIndex = MODS.indexOf(newMode);
        
        if (oldIndex !== -1 && newIndex !== -1) {
          const dir = newIndex > oldIndex ? 'forward' : 'backward';
          neBody.setAttribute('data-panel-dir', dir);
        }
        
        neBody.setAttribute('data-active-mod', newMode);
      }

      panels.forEach(panel => {
        if (panel.classList.contains(`ne-module-${newMode}`)) {
          panel.classList.remove('exiting');
          panel.classList.add('active');
        } else if (panel.classList.contains('active')) {
          panel.classList.remove('active');
          panel.classList.add('exiting');
          
          // Cleanup exiting class once transition finishes (560ms duration)
          const oldPanel = panel;
          setTimeout(() => {
            oldPanel.classList.remove('exiting');
          }, 600);
        } else {
          panel.classList.remove('active');
          panel.classList.remove('exiting');
        }
      });
    });
  });

  /* ── Dedicated Media/Lyrics Panel Integration ── */
  const audio = document.getElementById('mediaAudio');
  
  // Home Panel controls
  const playBtn = document.getElementById('playBtn');
  const pf = notch.querySelector('.ne-prog-fill');
  const tsRow = notch.querySelector('.ne-ts-row');
  const tsSpans = tsRow ? tsRow.querySelectorAll('.ne-ts') : [];
  const elapsedText = tsSpans[0];
  const durationText = tsSpans[1];
  const progTrack = notch.querySelector('.ne-prog-track');

  // Dedicated Media Panel controls
  const mediaPanelPlayBtn = document.getElementById('mediaPanelPlayBtn');
  const mediaPanelProgTrack = document.getElementById('mediaPanelProgTrack');
  const mediaPanelProgFill = document.getElementById('mediaPanelProgFill');
  const mediaPanelProgThumb = document.getElementById('mediaPanelProgThumb');
  const mediaPanelTimePos = document.getElementById('mediaPanelTimePos');
  const mediaPanelTimeDur = document.getElementById('mediaPanelTimeDur');
  const mediaPanelWaveform = document.getElementById('mediaPanelWaveform');
  const mediaPanelShuffleBtn = document.getElementById('mediaPanelShuffleBtn');
  const mediaPanelRepeatBtn = document.getElementById('mediaPanelRepeatBtn');
  const mediaPanelPrevBtn = document.getElementById('mediaPanelPrevBtn');
  const mediaPanelNextBtn = document.getElementById('mediaPanelNextBtn');
  
  // Lyrics scroller setup
  const lyricsScroller = document.getElementById('lyricsScroller');
  const lyricsData = [
    { time: 0, text: "♪ (Intro - Instrumental)" },
    { time: 8, text: "Thought I almost died in my dreams" },
    { time: 13, text: "Fighting for my life, I couldn't breathe" },
    { time: 19, text: "I'd fall back in a/into your arms" },
    { time: 24, text: "I'll never let you go without a fight" },
    { time: 30, text: "Oh, baby, where are you now when I need you most?" },
    { time: 36, text: "I'd give it all just to hold you close" },
    { time: 42, text: "Sorry that I broke your heart, your heart" },
    { time: 48, text: "Never intended to tear us apart" },
    { time: 54, text: "Oh, baby, where are you now when I need you most?" },
    { time: 60, text: "I'd give it all just to hold you close" },
    { time: 66, text: "♪ (Instrumental Break)" },
    { time: 78, text: "I'm running out of time" },
    { time: 84, text: "'Cause I can't get you out of my mind" },
    { time: 90, text: "I want you to be mine" },
    { time: 96, text: "I'm running out of time..." },
    { time: 102, text: "♪ (Guitar Solo)" },
    { time: 120, text: "Thought I almost died in my dreams" },
    { time: 126, text: "Oh, baby, where are you now..." },
    { time: 132, text: "I'll never let you go without a fight..." },
    { time: 138, text: "♪ (Outro)" }
  ];

  if (lyricsScroller) {
    lyricsScroller.innerHTML = lyricsData.map((line, idx) => `
      <div class="ne-lyric-line" data-idx="${idx}">${line.text}</div>
    `).join('');
  }

  let isPlaying = false;
  let mockCurrentTime = 0;
  const songDuration = 361; // 6:01 in seconds
  let timerId = null;
  let lastTickTime = 0;

  function formatTime(seconds) {
    if (isNaN(seconds) || seconds === Infinity) return '0:00';
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    return mins + ':' + String(secs).padStart(2, '0');
  }

  function updateLyrics(currentTime) {
    let activeIdx = 0;
    for (let i = 0; i < lyricsData.length; i++) {
      if (currentTime >= lyricsData[i].time) {
        activeIdx = i;
      } else {
        break;
      }
    }
    
    const lines = lyricsScroller ? lyricsScroller.querySelectorAll('.ne-lyric-line') : [];
    lines.forEach((line, idx) => {
      if (idx === activeIdx) {
        line.classList.add('active');
      } else {
        line.classList.remove('active');
      }
    });
    
    if (lyricsScroller) {
      const offset = 63 - (activeIdx * 32);
      lyricsScroller.style.transform = `translateY(${offset}px)`;
    }
  }

  // Waveform phase variable
  let wavePhase = 0;

  function updateWaveformPaths() {
    const w1 = document.getElementById('wavePath1');
    const w2 = document.getElementById('wavePath2');
    const w3 = document.getElementById('wavePath3');

    if (!isPlaying) {
      if (w1) w1.setAttribute('d', "M 0 16 L 344 16");
      if (w2) w2.setAttribute('d', "M 0 16 L 344 16");
      if (w3) w3.setAttribute('d', "M 0 16 L 344 16");
      return;
    }
    
    wavePhase = (wavePhase + 0.08) % (2.0 * Math.PI);
    
    if (w1) w1.setAttribute('d', generateWavePath(344, 48, 7.0, 1.3, wavePhase, 0.25));
    if (w2) w2.setAttribute('d', generateWavePath(344, 48, 10.0, 2.1, -wavePhase * 1.2, 0.3));
    if (w3) w3.setAttribute('d', generateWavePath(344, 48, 5.0, 2.8, wavePhase * 1.5, 0.15));
  }

  function generateWavePath(width, points, amplitude, cycles, phase, secondWaveRatio) {
    if (points === 0) return "";
    let path = "M 0 16";
    const pi = Math.PI;
    
    for (let i = 1; i <= points; i++) {
      const pct = i / points;
      const x = pct * width;
      
      // Envelope: window function to taper off at the edges
      const envelope = Math.sin(pct * pi);
      
      // Main wave component
      const mainWave = Math.sin(pct * pi * 2.0 * cycles + phase);
      
      // Secondary wave component to add dynamic organic complexity (liquid look)
      const secWave = Math.cos(pct * pi * 2.0 * cycles * 2.2 - phase * 0.7) * secondWaveRatio;
      
      const waveValue = mainWave + secWave;
      
      // Vertical center of the 32px height container is 16.0
      const y = 16.0 + amplitude * envelope * waveValue;
      
      path += ` L ${x.toFixed(2)} ${y.toFixed(2)}`;
    }
    
    return path;
  }

  function updateUI(currentTime, duration) {
    const pct = (currentTime / duration) * 100;
    
    // Update Home Panel progress
    if (pf) pf.style.width = pct + '%';
    if (elapsedText) elapsedText.textContent = formatTime(currentTime);
    
    // Update Media Panel progress
    if (mediaPanelProgFill) mediaPanelProgFill.style.width = pct + '%';
    if (mediaPanelProgThumb) mediaPanelProgThumb.style.left = pct + '%';
    if (mediaPanelTimePos) mediaPanelTimePos.textContent = formatTime(currentTime);
    if (mediaPanelTimeDur) mediaPanelTimeDur.textContent = formatTime(duration);
    
    // Update Lyrics Scroller
    updateLyrics(currentTime);
  }

  function startPlayback() {
    isPlaying = true;
    
    // Home Panel play states
    if (playBtn) {
      playBtn.title = 'Pause';
      const iconPause = playBtn.querySelector('.icon-pause');
      const iconPlay = playBtn.querySelector('.icon-play');
      if (iconPause) iconPause.style.display = '';
      if (iconPlay) iconPlay.style.display = 'none';
    }

    // Media Panel play states
    if (mediaPanelPlayBtn) {
      mediaPanelPlayBtn.title = 'Pause';
      const iconPause = mediaPanelPlayBtn.querySelector('.icon-media-panel-pause');
      const iconPlay = mediaPanelPlayBtn.querySelector('.icon-media-panel-play');
      if (iconPause) iconPause.style.display = '';
      if (iconPlay) iconPlay.style.display = 'none';
    }

    // Waveform container
    if (mediaPanelWaveform) mediaPanelWaveform.classList.add('active');

    if (audio) {
      audio.play().catch(err => {
        console.warn("Audio playback failed or was blocked (using visual fallback):", err);
      });
    }

    lastTickTime = performance.now();
    if (timerId) cancelAnimationFrame(timerId);
    
    function tick() {
      if (!isPlaying) return;
      const now = performance.now();
      const dt = (now - lastTickTime) / 1000;
      lastTickTime = now;

      let current = mockCurrentTime;
      let duration = songDuration;

      if (audio && !audio.paused && audio.duration) {
        current = audio.currentTime;
        duration = audio.duration;
        mockCurrentTime = current;
      } else {
        mockCurrentTime = Math.min(duration, mockCurrentTime + dt);
        current = mockCurrentTime;
      }

      updateUI(current, duration);
      updateWaveformPaths();

      if (durationText && audio && audio.duration) {
        durationText.textContent = formatTime(audio.duration);
      }

      if (current >= duration) {
        pausePlayback(true);
      } else {
        timerId = requestAnimationFrame(tick);
      }
    }
    timerId = requestAnimationFrame(tick);
  }

  function pausePlayback(reset = false) {
    isPlaying = false;
    
    // Home Panel pause states
    if (playBtn) {
      playBtn.title = 'Play';
      const iconPause = playBtn.querySelector('.icon-pause');
      const iconPlay = playBtn.querySelector('.icon-play');
      if (iconPause) iconPause.style.display = 'none';
      if (iconPlay) iconPlay.style.display = '';
    }

    // Media Panel pause states
    if (mediaPanelPlayBtn) {
      mediaPanelPlayBtn.title = 'Play';
      const iconPause = mediaPanelPlayBtn.querySelector('.icon-media-panel-pause');
      const iconPlay = mediaPanelPlayBtn.querySelector('.icon-media-panel-play');
      if (iconPause) iconPause.style.display = 'none';
      if (iconPlay) iconPlay.style.display = '';
    }

    // Waveform container
    if (mediaPanelWaveform) mediaPanelWaveform.classList.remove('active');

    if (audio) {
      audio.pause();
      if (reset) audio.currentTime = 0;
    }

    const w1 = document.getElementById('wavePath1');
    const w2 = document.getElementById('wavePath2');
    const w3 = document.getElementById('wavePath3');
    if (w1) w1.setAttribute('d', "M 0 16 L 344 16");
    if (w2) w2.setAttribute('d', "M 0 16 L 344 16");
    if (w3) w3.setAttribute('d', "M 0 16 L 344 16");

    if (reset) {
      mockCurrentTime = 0;
      updateUI(0, songDuration);
    }

    if (timerId) {
      cancelAnimationFrame(timerId);
      timerId = null;
    }
  }

  // Play button click handlers
  if (playBtn) {
    playBtn.addEventListener('click', e => {
      e.stopPropagation();
      if (isPlaying) {
        pausePlayback();
      } else {
        startPlayback();
      }
    });
  }

  if (mediaPanelPlayBtn) {
    mediaPanelPlayBtn.addEventListener('click', e => {
      e.stopPropagation();
      if (isPlaying) {
        pausePlayback();
      } else {
        startPlayback();
      }
    });
  }

  // Prev / Next skips
  if (mediaPanelPrevBtn) {
    mediaPanelPrevBtn.addEventListener('click', e => {
      e.stopPropagation();
      mockCurrentTime = 0;
      if (audio) audio.currentTime = 0;
      updateUI(0, songDuration);
    });
  }

  if (mediaPanelNextBtn) {
    mediaPanelNextBtn.addEventListener('click', e => {
      e.stopPropagation();
      mockCurrentTime = 0;
      if (audio) audio.currentTime = 0;
      updateUI(0, songDuration);
    });
  }

  // Shuffle & Repeat toggles
  if (mediaPanelShuffleBtn) {
    let shuffleActive = false;
    mediaPanelShuffleBtn.addEventListener('click', e => {
      e.stopPropagation();
      shuffleActive = !shuffleActive;
      mediaPanelShuffleBtn.classList.toggle('active', shuffleActive);
    });
  }

  if (mediaPanelRepeatBtn) {
    let repeatActive = false;
    mediaPanelRepeatBtn.addEventListener('click', e => {
      e.stopPropagation();
      repeatActive = !repeatActive;
      mediaPanelRepeatBtn.classList.toggle('active', repeatActive);
    });
  }

  // Audio elements events
  if (audio) {
    audio.addEventListener('play', () => {
      if (!isPlaying) startPlayback();
    });
    audio.addEventListener('pause', () => {
      if (isPlaying) pausePlayback();
    });
    audio.addEventListener('ended', () => {
      pausePlayback(true);
    });
    audio.addEventListener('loadedmetadata', () => {
      if (durationText) durationText.textContent = formatTime(audio.duration);
    });
    audio.addEventListener('durationchange', () => {
      if (durationText) durationText.textContent = formatTime(audio.duration);
    });
  }

  // Scrubbing handlers
  if (progTrack) {
    progTrack.addEventListener('click', e => {
      e.stopPropagation();
      const rect = progTrack.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const pct = Math.max(0, Math.min(1, clickX / rect.width));
      
      let duration = songDuration;
      if (audio && audio.duration) duration = audio.duration;

      const newTime = pct * duration;
      mockCurrentTime = newTime;
      updateUI(newTime, duration);

      if (audio) audio.currentTime = newTime;
    });
  }

  if (mediaPanelProgTrack) {
    mediaPanelProgTrack.addEventListener('click', e => {
      e.stopPropagation();
      const rect = mediaPanelProgTrack.getBoundingClientRect();
      const clickX = e.clientX - rect.left;
      const pct = Math.max(0, Math.min(1, clickX / rect.width));
      
      let duration = songDuration;
      if (audio && audio.duration) duration = audio.duration;

      const newTime = pct * duration;
      mockCurrentTime = newTime;
      updateUI(newTime, duration);

      if (audio) audio.currentTime = newTime;
    });
  }

  /* ── Caffeine toggle ── */
  const cafBtn = document.getElementById('cafBtn');
  if (cafBtn) {
    let cafOn = false;
    cafBtn.addEventListener('click', e => {
      e.stopPropagation();
      cafOn = !cafOn;
      cafBtn.classList.toggle('caf-on', cafOn);
    });
  }

  /* ── Live calendar & Google Account Simulation ── */
  let googleConnected = false;
  let googleBusy = false;
  let selectedDayOffset = 0; // offset relative to today: -2 to 4 (0 is today)

  const mockEvents = {
    "0": [
      { title: "Project Sync meeting", time: "10:00 AM - 10:45 AM" },
      { title: "Pair Programming with Antigravity", time: "2:00 PM - 3:30 PM" }
    ],
    "1": [
      { title: "Weekly Strategy Review", time: "11:00 AM - 12:00 PM" }
    ],
    "2": [
      { title: "Design Critique Session", time: "4:00 PM - 5:00 PM" }
    ]
  };

  const DAY_SHORT = ['SUN','MON','TUE','WED','THU','FRI','SAT'];
  const DAY_LONG  = ['Sunday','Monday','Tuesday','Wednesday','Thursday','Friday','Saturday'];
  const MON_LONG  = ['January','February','March','April','May','June','July','August','September','October','November','December'];
  const MON_SHORT = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];

  function playHomeSound(soundName) {
    try {
      const audio = new Audio(`assets/sounds/${soundName}.wav`);
      audio.volume = 0.4;
      audio.play().catch(err => console.log('Home audio blocked:', err));
    } catch (e) {
      console.warn('Home audio error:', e);
    }
  }

  function renderEvents() {
    const cardEl = document.getElementById('calEventsCard');
    if (!cardEl) return;

    if (!googleConnected) {
      cardEl.innerHTML = `
        <div class="ne-google-connect-card">
          <div class="ne-google-logo-row">
            <svg class="ne-google-svg" viewBox="0 0 48 48" width="24" height="24">
              <path fill="#EA4335" d="M 24 9.5 C 27.5 9.5 30.6 10.7 33.1 13.1 L 39.9 6.3 C 35.8 2.5 30.4 0.2 24 0.2 C 14.7 0.2 6.7 5.5 2.8 13.2 L 10.7 19.3 C 12.6 13.5 17.9 9.5 24 9.5 Z"/>
              <path fill="#4285F4" d="M 46.9 24.5 C 46.9 22.9 46.8 21.7 46.5 20.4 L 24 20.4 L 24 28.7 L 37.2 28.7 C 36.9 30.8 35.5 34 32.3 36.2 L 40 42.2 C 44.5 38 46.9 31.9 46.9 24.5 Z"/>
              <path fill="#FBBC05" d="M 10.7 28.7 C 10.2 27.2 9.9 25.6 9.9 24 C 9.9 22.4 10.2 20.8 10.7 19.3 L 2.8 13.2 C 1.1 16.5 0.2 20.1 0.2 24 C 0.2 27.9 1.1 31.5 2.8 34.8 L 10.7 28.7 Z"/>
              <path fill="#34A853" d="M 24 47.8 C 30.4 47.8 35.8 45.7 39.8 42.1 L 32.1 36.1 C 30 37.5 27.3 38.5 24 38.5 C 17.9 38.5 12.7 34.5 10.8 28.9 L 2.9 35 C 6.7 42.5 14.7 47.8 24 47.8 Z"/>
            </svg>
            <svg class="ne-cal-icon-deco" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <rect x="3" y="4" width="18" height="18" rx="2" ry="2"/>
              <line x1="16" y1="2" x2="16" y2="6"/>
              <line x1="8" y1="2" x2="8" y2="6"/>
              <line x1="3" y1="10" x2="21" y2="10"/>
            </svg>
          </div>
          <div class="ne-google-title-group">
            <div class="ne-google-connect-title">Connect Google Account</div>
            <div class="ne-google-connect-desc">${googleBusy ? "Authorizing access to calendar..." : "Sign in to sync your calendar."}</div>
          </div>
          <button class="ne-google-signin-btn" id="googleSignInBtn" ${googleBusy ? "disabled" : ""}>
            <span class="ne-signin-btn-text">${googleBusy ? "Signing in..." : "Sign in"}</span>
          </button>
        </div>
      `;

      const signinBtn = document.getElementById('googleSignInBtn');
      if (signinBtn) {
        signinBtn.addEventListener('click', e => {
          e.stopPropagation();
          triggerGoogleConnect();
        });
      }
    } else {
      const evts = mockEvents[String(selectedDayOffset)] || [];
      
      let eventsHTML = '';
      if (evts.length === 0) {
        eventsHTML = `
          <div class="ne-events-empty-connected">
            <div class="ne-events-empty-connected-title">No events on this date</div>
            <div class="ne-events-empty-connected-sub">Enjoy your free day!</div>
          </div>
        `;
      } else {
        eventsHTML = `
          <div class="ne-events-scroll">
            ${evts.map(evt => `
              <div class="ne-event-item">
                <div class="ne-event-item-accent"></div>
                <div class="ne-event-title">${evt.title}</div>
                <div class="ne-event-time">${evt.time}</div>
              </div>
            `).join('')}
          </div>
        `;
      }

      cardEl.innerHTML = `
        <div class="ne-cal-connected-container">
          <div class="ne-cal-connected-header">
            <span>GOOGLE CALENDAR</span>
            <button class="ne-cal-signout-btn" id="googleSignOutBtn">Sign out</button>
          </div>
          ${eventsHTML}
        </div>
      `;

      const signoutBtn = document.getElementById('googleSignOutBtn');
      if (signoutBtn) {
        signoutBtn.addEventListener('click', e => {
          e.stopPropagation();
          playHomeSound('close_003');
          googleConnected = false;
          renderEvents();
        });
      }
    }
  }

  function triggerGoogleConnect() {
    if (googleBusy) return;
    googleBusy = true;
    playHomeSound('click_003');
    renderEvents();

    setTimeout(() => {
      googleConnected = true;
      googleBusy = false;
      playHomeSound('confirmation_004'); // success chime
      renderEvents();
    }, 1200);
  }

  function buildCalendar() {
    const now = new Date();
    const monthEl = document.getElementById('calMonth');
    const yearEl  = document.getElementById('calYear');
    const scrubberEl = document.getElementById('calScrubber');
    const selectedLbl = document.getElementById('calSelectedDayLabel');

    if (monthEl) monthEl.textContent = MON_LONG[now.getMonth()];
    if (yearEl)  yearEl.textContent  = now.getFullYear();

    let scrubberHTML = '';
    const startOffset = -2;
    const endOffset = 4;
    
    for (let offset = startOffset; offset <= endOffset; offset++) {
      const d = new Date(now);
      d.setDate(now.getDate() + offset);
      
      const dayName = DAY_SHORT[d.getDay()];
      const dayNum = String(d.getDate()).padStart(2, '0');
      const isToday = offset === 0;
      const isSelected = offset === selectedDayOffset;

      scrubberHTML += `
        <div class="ne-cal-scrub-item${isSelected ? ' selected' : ''}${isToday ? ' today' : ''}" data-offset="${offset}">
          <span class="ne-cal-scrub-day">${dayName}</span>
          <div class="ne-cal-scrub-num-wrap">
            <span class="ne-cal-scrub-num">${dayNum}</span>
          </div>
        </div>
      `;
    }

    if (scrubberEl) {
      scrubberEl.innerHTML = scrubberHTML;

      const items = scrubberEl.querySelectorAll('.ne-cal-scrub-item');
      items.forEach(item => {
        item.addEventListener('click', e => {
          e.stopPropagation();
          const offset = parseInt(item.dataset.offset);
          if (offset === selectedDayOffset) return;
          
          selectedDayOffset = offset;
          playHomeSound('click_003');
          buildCalendar();
          renderEvents();
        });
      });
    }

    const targetDate = new Date(now);
    targetDate.setDate(now.getDate() + selectedDayOffset);
    const dayLongName = DAY_LONG[targetDate.getDay()];
    const dayNum = targetDate.getDate();
    const monShortName = MON_SHORT[targetDate.getMonth()];
    const targetYear = targetDate.getFullYear();

    if (selectedLbl) {
      selectedLbl.textContent = `${dayLongName}, ${monShortName} ${dayNum}, ${targetYear}`;
    }
  }

  buildCalendar();
  renderEvents();

  /* ── Taskbar clock ── */
  function pad(n) { return String(n).padStart(2,'0'); }
  function tickClock() {
    const now  = new Date();
    const t    = `${pad(now.getHours())}:${pad(now.getMinutes())}`;
    const tbT  = document.getElementById('simTaskbarTime');
    const tbD  = document.getElementById('simTaskbarDate');
    if (tbT) tbT.textContent = t;
    if (tbD) tbD.textContent = `${now.getMonth()+1}/${now.getDate()}/${now.getFullYear()}`;
  }
  tickClock();
  setInterval(tickClock, 1000);

  /* ── Battery indicator (fixed at 100%) ── */
  const battFill = document.getElementById('battFill');
  const battPct  = document.getElementById('battPct');
  if (battFill) {
    battFill.style.width = '100%';
    battFill.style.background = '#10b981';
  }
  if (battPct) {
    battPct.textContent = '100%';
  }

  /* ============================================================
     DROP SHELF MODULE ENGINE
     ============================================================ */
  const shelfModule = document.getElementById('shelfModule');
  const shelfShareTarget = document.getElementById('shelfShareTarget');
  const shelfProviderIcon = document.getElementById('shelfProviderIcon');
  const shelfProviderName = document.getElementById('shelfProviderName');
  const shelfShareNotice = document.getElementById('shelfShareNotice');
  const shelfBorderLeftRect = document.getElementById('shelfBorderLeftRect');
  const shelfBorderRightRect = document.getElementById('shelfBorderRightRect');
  const shelfToolbar = document.getElementById('shelfToolbar');
  const shelfTitle = document.getElementById('shelfTitle');
  const shelfClearBtn = document.getElementById('shelfClearBtn');
  const shelfEmptyState = document.getElementById('shelfEmptyState');
  const shelfCardsScroll = document.getElementById('shelfCardsScroll');
  const shelfCardsContainer = document.getElementById('shelfCardsContainer');

  // Provider list mapping colors and assets
  const shelfProviders = [
    { id: 'localsend', name: 'LocalSend', color: '#00D1B2', icon: 'assets/localsend.png', sizeClass: 'localsend-size' },
    { id: 'quickshare', name: 'Quick Share', color: '#1A73E8', icon: 'assets/quickshare.svg', sizeClass: '' },
    { id: 'kdeconnect', name: 'KDE Connect', color: '#8A2BE2', icon: 'assets/kdeconnect.svg', sizeClass: '' }
  ];
  let currentProviderIndex = 0;
  let shelfItems = [];

  // Audio helper
  function playShelfSound(soundName) {
    try {
      const audio = new Audio(`assets/sounds/${soundName}.wav`);
      audio.volume = 0.5;
      audio.play().catch(err => console.log('Audio playback blocked:', err));
    } catch (e) {
      console.warn('Sound play error:', e);
    }
  }

  // Render provider details
  function updateProviderUI() {
    const prov = shelfProviders[currentProviderIndex];
    if (shelfProviderName) shelfProviderName.textContent = prov.name;
    if (shelfProviderIcon) {
      shelfProviderIcon.src = prov.icon;
      if (prov.sizeClass) {
        shelfProviderIcon.className = prov.sizeClass;
      } else {
        shelfProviderIcon.className = '';
      }
    }
    // Set left border default stroke
    if (shelfBorderLeftRect) {
      shelfBorderLeftRect.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');
    }
  }

  // Cycle providers on click
  if (shelfShareTarget) {
    shelfShareTarget.addEventListener('click', e => {
      e.stopPropagation();
      currentProviderIndex = (currentProviderIndex + 1) % shelfProviders.length;
      updateProviderUI();
      playShelfSound('click_003');
    });
  }

  // Format byte sizing
  function formatBytes(bytes) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  // Clear notice simulation timeout
  let noticeTimeoutId = null;

  // Render cards lists
  function renderShelfCards() {
    if (!shelfCardsContainer || !shelfTitle || !shelfToolbar || !shelfEmptyState || !shelfCardsScroll) return;

    const count = shelfItems.length;
    shelfTitle.textContent = `Shelf (${count})`;

    if (count === 0) {
      shelfToolbar.style.display = 'none';
      shelfCardsScroll.style.display = 'none';
      shelfEmptyState.style.display = 'flex';
      return;
    }

    shelfToolbar.style.display = 'flex';
    shelfCardsScroll.style.display = 'block';
    shelfEmptyState.style.display = 'none';

    shelfCardsContainer.innerHTML = shelfItems.map((item, idx) => {
      const typeIconSrc = item.isImage ? '' : (item.isVideo ? 'assets/waveform.svg' : 'assets/books.svg');
      const iconColorClass = item.isVideo ? 'style="filter: invert(53%) sepia(87%) saturate(1915%) hue-rotate(114deg) brightness(101%) contrast(101%);"' : 'style="filter: invert(86%) sepia(35%) saturate(1106%) hue-rotate(352deg) brightness(102%) contrast(104%);"'; // turquoise vs gold

      return `
        <div class="ne-shelf-card" data-id="${item.id}">
          ${item.isImage && item.thumbnail ? `
            <div class="ne-card-bg-preview">
              <img src="${item.thumbnail}" alt="preview" />
            </div>
          ` : ''}
          <div class="ne-card-content">
            <div class="ne-card-row-top">
              <div class="ne-card-type-icon">
                ${item.isImage && item.thumbnail ? `
                  <img src="${item.thumbnail}" alt="icon" style="border-radius: 2px;" />
                ` : `
                  <img src="${typeIconSrc}" alt="icon" ${iconColorClass} />
                `}
              </div>
              <button class="ne-card-close-btn" title="Remove" onclick="window.removeShelfItem('${item.id}', event)">&times;</button>
            </div>
            <div class="ne-card-row-mid">
              <div class="ne-card-filename" title="${item.name}">${item.name}</div>
              <div class="ne-card-filesize">${item.sizeStr}</div>
            </div>
            <div class="ne-card-row-bot">
              <button class="ne-card-action-btn" onclick="window.openShelfItem('${item.path}', event)">Open</button>
              <button class="ne-card-action-btn" onclick="window.copyShelfItemPath('${item.path}', event)">Copy</button>
              <button class="ne-card-action-btn btn-remove" onclick="window.removeShelfItem('${item.id}', event)">Remove</button>
            </div>
          </div>
        </div>
      `;
    }).join('');
  }

  // Global functions exposed to inline click events
  window.removeShelfItem = function(id, e) {
    if (e) e.stopPropagation();
    shelfItems = shelfItems.filter(item => item.id !== id);
    renderShelfCards();
    playShelfSound('minimize_004');
  };

  window.openShelfItem = function(path, e) {
    if (e) e.stopPropagation();
    alert(`Opening file from path: ${path}`);
    playShelfSound('click_003');
  };

  window.copyShelfItemPath = function(path, e) {
    if (e) e.stopPropagation();
    navigator.clipboard.writeText(path).then(() => {
      // Temporary notice
      const oldNotice = shelfShareNotice.textContent;
      const oldClass = shelfShareNotice.className;
      
      shelfShareNotice.textContent = "Path copied!";
      shelfShareNotice.className = "ne-share-notice sent";
      
      setTimeout(() => {
        if (shelfShareNotice.textContent === "Path copied!") {
          shelfShareNotice.textContent = oldNotice;
          shelfShareNotice.className = oldClass;
        }
      }, 1500);
      playShelfSound('click_003');
    }).catch(err => {
      console.error('Clipboard copy failed:', err);
    });
  };

  if (shelfClearBtn) {
    shelfClearBtn.addEventListener('click', e => {
      e.stopPropagation();
      shelfItems = [];
      renderShelfCards();
      playShelfSound('close_003');
    });
  }

  // Drag Drop listeners
  const ravenNotch = document.getElementById('ravenNotch');
  
  if (ravenNotch) {
    // Check if dragging over Left (Share target) or Right (Storage target)
    function getDragTarget(e) {
      if (!shelfModule || !shelfModule.classList.contains('active')) {
        // If shelf is not active, everything goes to the right storage by default
        return 'right';
      }
      
      const leftRect = shelfShareTarget.getBoundingClientRect();
      const clientX = e.clientX;
      
      if (clientX >= leftRect.left && clientX <= leftRect.right) {
        return 'left';
      }
      return 'right';
    }

    window.addEventListener('dragenter', e => {
      e.preventDefault();
      // Expand notch and switch to shelf
      ravenNotch.classList.add('drag-active');
      
      // Select the shelf tab
      const shelfTab = ravenNotch.querySelector('[data-mod="shelf"]');
      if (shelfTab && !shelfTab.classList.contains('active')) {
        shelfTab.click();
      }
    });

    window.addEventListener('dragover', e => {
      e.preventDefault();
      
      const dragTarget = getDragTarget(e);
      const activeProv = shelfProviders[currentProviderIndex];

      if (dragTarget === 'left') {
        if (shelfBorderLeftRect) shelfBorderLeftRect.setAttribute('stroke', activeProv.color);
        if (shelfBorderRightRect) shelfBorderRightRect.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');
      } else {
        if (shelfBorderLeftRect) shelfBorderLeftRect.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');
        if (shelfBorderRightRect) shelfBorderRightRect.setAttribute('stroke', '#00FF88');
      }
    });

    // We can also bind drag events specifically on ravenNotch to catch leave events
    ravenNotch.addEventListener('dragleave', e => {
      // Only clear if mouse left the notch boundaries
      const rect = ravenNotch.getBoundingClientRect();
      if (e.clientX < rect.left || e.clientX > rect.right || e.clientY < rect.top || e.clientY > rect.bottom) {
        ravenNotch.classList.remove('drag-active');
        if (shelfBorderLeftRect) shelfBorderLeftRect.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');
        if (shelfBorderRightRect) shelfBorderRightRect.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');
      }
    });

    window.addEventListener('drop', e => {
      e.preventDefault();
      ravenNotch.classList.remove('drag-active');
      if (shelfBorderLeftRect) shelfBorderLeftRect.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');
      if (shelfBorderRightRect) shelfBorderRightRect.setAttribute('stroke', 'rgba(255, 255, 255, 0.15)');

      const files = e.dataTransfer.files;
      if (!files || files.length === 0) return;

      const dragTarget = getDragTarget(e);
      const activeProv = shelfProviders[currentProviderIndex];

      if (dragTarget === 'left') {
        // Simulate Share
        const firstFile = files[0];
        if (noticeTimeoutId) clearTimeout(noticeTimeoutId);

        if (shelfShareNotice) {
          shelfShareNotice.textContent = `Sending ${firstFile.name}...`;
          shelfShareNotice.className = "ne-share-notice info";
        }
        playShelfSound('maximize_004');

        noticeTimeoutId = setTimeout(() => {
          if (shelfShareNotice) {
            shelfShareNotice.textContent = "Sent!";
            shelfShareNotice.className = "ne-share-notice sent";
          }
          playShelfSound('confirmation_002');

          noticeTimeoutId = setTimeout(() => {
            if (shelfShareNotice && shelfShareNotice.textContent === "Sent!") {
              shelfShareNotice.textContent = "";
              shelfShareNotice.className = "ne-share-notice";
            }
          }, 2000);
        }, 1500);

      } else {
        // Add to shelf storage
        let processedCount = 0;
        
        for (let i = 0; i < files.length; i++) {
          const file = files[i];
          const isImg = file.type.startsWith('image/');
          const isVid = file.type.startsWith('video/');

          const item = {
            id: Date.now() + Math.random().toString(36).substr(2, 5),
            name: file.name,
            sizeStr: formatBytes(file.size),
            isImage: isImg,
            isVideo: isVid,
            thumbnail: '',
            path: `C:\\Users\\Desktop\\${file.name}`
          };

          if (isImg) {
            const reader = new FileReader();
            reader.onload = function(evt) {
              item.thumbnail = evt.target.result;
              shelfItems.push(item);
              processedCount++;
              if (processedCount === files.length) {
                renderShelfCards();
                playShelfSound('confirmation_004');
              }
            };
            reader.readAsDataURL(file);
          } else {
            shelfItems.push(item);
            processedCount++;
            if (processedCount === files.length) {
              renderShelfCards();
              playShelfSound('confirmation_004');
            }
          }
        }
      }
    });
  }

  // Run initialization
  updateProviderUI();
  renderShelfCards();

  /* ============================================================
     CLOCK MODULE ENGINE
     ============================================================ */
  
  // 1. Generate Analog Clock ticks dynamically (mathematically perfect for 154x154 viewBox)
  const ticksGroup = document.getElementById('clockTicks');
  if (ticksGroup) {
    ticksGroup.innerHTML = '';
    for (let i = 0; i < 60; i++) {
      const angle = i * 6;
      const isHour = i % 5 === 0;
      const tick = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      const r1 = isHour ? 60 : 66; // 11px hour ticks, 5px minute ticks
      const r2 = 71;                // outermost radius limit
      const rad = (angle - 90) * Math.PI / 180;
      tick.setAttribute('x1', String(77 + r1 * Math.cos(rad)));
      tick.setAttribute('y1', String(77 + r1 * Math.sin(rad)));
      tick.setAttribute('x2', String(77 + r2 * Math.cos(rad)));
      tick.setAttribute('y2', String(77 + r2 * Math.sin(rad)));
      tick.setAttribute('class', isHour ? 'hour-tick' : 'min-tick');
      ticksGroup.appendChild(tick);
    }
  }

  // Helper to pad numbers
  function padZero(n, len = 2) {
    return String(n).padStart(len, '0');
  }

  // 2. Analog and Digital Clock Update Loop
  const anaHour = document.getElementById('hourHand');
  const anaMin  = document.getElementById('minHand');
  const anaSec  = document.getElementById('secHand');
  const digTime = document.getElementById('neClockDigTime');
  const digAmPm = document.getElementById('neClockDigAmPm');
  const digSec  = document.getElementById('neClockDigSec');

  function tickClockPanel() {
    const now = new Date();

    // Analog clock hands rotations
    const hours = now.getHours();
    const minutes = now.getMinutes();
    const seconds = now.getSeconds();
    const ms = now.getMilliseconds();

    const secDeg = seconds * 6;
    const minDeg = (minutes * 6) + (seconds * 0.1);
    const hrDeg  = ((hours % 12) * 30) + (minutes * 0.5);

    if (anaSec)  anaSec.style.transform  = `rotate(${secDeg}deg)`;
    if (anaMin)  anaMin.style.transform  = `rotate(${minDeg}deg)`;
    if (anaHour) anaHour.style.transform = `rotate(${hrDeg}deg)`;

    // Digital clock display
    let dispHour = hours % 12;
    if (dispHour === 0) dispHour = 12;
    const dispAmPm = hours >= 12 ? 'PM' : 'AM';

    if (digTime) digTime.textContent = `${dispHour}:${padZero(minutes)}`;
    if (digAmPm) digAmPm.textContent = dispAmPm;
    if (digSec)  digSec.textContent  = padZero(seconds);

    // 3. World Clocks Updates
    updateWorldClocks(now);
  }

  // World Clocks mapping with offsets
  const worldClocks = [
    { id: 'NY', offset: -4, hourHandId: 'miniClockNY', timeId: 'worldTimeNY', dateId: 'worldDateNY', visible: true, name: 'New York', rowId: 'worldRowNY' },
    { id: 'London', offset: 1, hourHandId: 'miniClockLondon', timeId: 'worldTimeLondon', dateId: 'worldDateLondon', visible: true, name: 'London', rowId: 'worldRowLondon' },
    { id: 'Tokyo', offset: 9, hourHandId: 'miniClockTokyo', timeId: 'worldTimeTokyo', dateId: 'worldDateTokyo', visible: true, name: 'Tokyo', rowId: 'worldRowTokyo' },
    { id: 'Delhi', offset: 5.5, hourHandId: 'miniClockDelhi', timeId: 'worldTimeDelhi', dateId: 'worldDateDelhi', visible: false, name: 'New Delhi', rowId: 'worldRowDelhi' },
    { id: 'Sydney', offset: 10, hourHandId: 'miniClockSydney', timeId: 'worldTimeSydney', dateId: 'worldDateSydney', visible: false, name: 'Sydney', rowId: 'worldRowSydney' },
    { id: 'Paris', offset: 1, hourHandId: 'miniClockParis', timeId: 'worldTimeParis', dateId: 'worldDateParis', visible: false, name: 'Paris', rowId: 'worldRowParis' },
    { id: 'Dubai', offset: 4, hourHandId: 'miniClockDubai', timeId: 'worldTimeDubai', dateId: 'worldDateDubai', visible: false, name: 'Dubai', rowId: 'worldRowDubai' },
    { id: 'Singapore', offset: 8, hourHandId: 'miniClockSingapore', timeId: 'worldTimeSingapore', dateId: 'worldDateSingapore', visible: false, name: 'Singapore', rowId: 'worldRowSingapore' }
  ];

  const MONTH_NAMES_SHORT = ['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];

  function updateWorldClocks(nowSystem) {
    const utcTime = nowSystem.getTime() + (nowSystem.getTimezoneOffset() * 60000);
    const systemSeconds = nowSystem.getSeconds() + (nowSystem.getMilliseconds() / 1000);

    worldClocks.forEach(zone => {
      const rowEl = document.getElementById(zone.rowId);
      if (rowEl) {
        rowEl.style.display = zone.visible ? 'flex' : 'none';
      }

      if (!zone.visible) return;

      const localTime = new Date(utcTime + (3600000 * zone.offset));

      const hrs = localTime.getHours();
      const mins = localTime.getMinutes();

      const miniHourHand = document.querySelector(`#${zone.hourHandId} .mclock-hour`);
      const miniMinHand  = document.querySelector(`#${zone.hourHandId} .mclock-min`);
      const miniSecHand  = document.querySelector(`#${zone.hourHandId} .mclock-sec`);
      
      const hrDeg  = ((hrs % 12) * 30) + (mins * 0.5);
      const minDeg = mins * 6;
      const secDeg = localTime.getSeconds() * 6;

      if (miniHourHand) miniHourHand.style.transform = `rotate(${hrDeg}deg)`;
      if (miniMinHand)  miniMinHand.style.transform  = `rotate(${minDeg}deg)`;
      if (miniSecHand)  miniSecHand.style.transform  = `rotate(${secDeg}deg)`;

      let dispHr = hrs % 12;
      if (dispHr === 0) dispHr = 12;
      const ampm = hrs >= 12 ? 'PM' : 'AM';
      
      const dateStr = `${MONTH_NAMES_SHORT[localTime.getMonth()]} ${localTime.getDate()}`;

      const tEl = document.getElementById(zone.timeId);
      const dEl = document.getElementById(zone.dateId);

      if (tEl) tEl.innerHTML = `${dispHr}:${padZero(mins)}<span class="ne-world-ampm">${ampm}</span>`;
      if (dEl) dEl.textContent = dateStr;
    });
  }

  // Ticking interval for clocks
  setInterval(tickClockPanel, 200);
  tickClockPanel();

  // Populate Add city dropdown dynamically
  const worldAddBtn = document.getElementById('worldAddBtn');
  const worldAddDropdown = document.getElementById('worldAddDropdown');

  function renderAddDropdown() {
    if (!worldAddDropdown) return;
    const inactiveClocks = worldClocks.filter(zone => !zone.visible);
    if (inactiveClocks.length === 0) {
      worldAddDropdown.innerHTML = '<div class="ne-world-add-item" style="color: rgba(255,255,255,0.3); pointer-events: none;">No more cities</div>';
    } else {
      worldAddDropdown.innerHTML = inactiveClocks.map(zone => `
        <div class="ne-world-add-item" data-id="${zone.id}">${zone.name}</div>
      `).join('');
    }
  }

  if (worldAddDropdown) {
    worldAddDropdown.addEventListener('click', e => {
      const item = e.target.closest('.ne-world-add-item');
      if (item) {
        const zoneId = item.getAttribute('data-id');
        const zone = worldClocks.find(z => z.id === zoneId);
        if (zone) {
          zone.visible = true;
          updateWorldClocks(new Date());
          renderAddDropdown();
          worldAddDropdown.classList.remove('active');
        }
      }
    });
  }

  if (worldAddBtn && worldAddDropdown) {
    worldAddBtn.addEventListener('click', e => {
      e.stopPropagation();
      renderAddDropdown();
      worldAddDropdown.classList.toggle('active');
    });
  }

  document.addEventListener('click', () => {
    if (worldAddDropdown) worldAddDropdown.classList.remove('active');
  });

  // Bind Delete buttons on world clock list
  const worldList = document.querySelector('.ne-world-list');
  if (worldList) {
    worldList.addEventListener('click', e => {
      const deleteBtn = e.target.closest('.ne-world-delete-btn');
      if (deleteBtn) {
        const cityCode = deleteBtn.getAttribute('data-city');
        const zone = worldClocks.find(z => z.id === cityCode);
        if (zone) {
          zone.visible = false;
          updateWorldClocks(new Date());
          renderAddDropdown();
        }
      }
    });
  }

  // 4. Sub-mode switching (TIMER, LAP, STOPWATCH)
  const modeSpans = notch.querySelectorAll('.ne-clock-mode');
  const viewTimer = document.getElementById('viewTimer');
  const viewLap = document.getElementById('viewLap');
  const viewStopwatch = document.getElementById('viewStopwatch');

  let activeMode = 'timer';

  modeSpans.forEach(span => {
    span.addEventListener('click', e => {
      e.stopPropagation();
      modeSpans.forEach(s => s.classList.remove('active'));
      span.classList.add('active');

      const mode = span.dataset.clockMode;
      activeMode = mode;

      [viewTimer, viewLap, viewStopwatch].forEach(v => {
        if (v) v.classList.remove('active');
      });

      if (mode === 'timer' && viewTimer) viewTimer.classList.add('active');
      if (mode === 'lap' && viewLap) viewLap.classList.add('active');
      if (mode === 'stopwatch' && viewStopwatch) viewStopwatch.classList.add('active');

      updateControlsUI();
    });
  });

  const clockPlayBtn = document.getElementById('clockPlayBtn');
  const clockResetBtn = document.getElementById('clockResetBtn');
  const clockLapBtn = document.getElementById('clockLapBtn');

  function updateControlsUI() {
    if (!clockPlayBtn) return;
    
    const iconPlay = clockPlayBtn.querySelector('.icon-clock-play');
    const iconPause = clockPlayBtn.querySelector('.icon-clock-pause');

    let isRunning = false;
    if (activeMode === 'timer') {
      isRunning = timerInterval !== null;
      if (clockLapBtn) clockLapBtn.style.opacity = '0.3';
    } else if (activeMode === 'stopwatch' || activeMode === 'lap') {
      isRunning = stopwatchInterval !== null;
      if (clockLapBtn) clockLapBtn.style.opacity = '1';
    }

    if (iconPlay) iconPlay.style.display = isRunning ? 'none' : '';
    if (iconPause) iconPause.style.display = isRunning ? '' : 'none';
  }

  // 5. Timer Logic & Set Overlay Panel
  let defaultTimerTime = 25 * 60 + 30;
  let timerTimeLeft = defaultTimerTime;
  let timerInterval = null;
  const timerValText = document.getElementById('timerVal');
  const timerInputPanel = document.getElementById('timerInputPanel');
  const timerInputH = document.getElementById('timerInputH');
  const timerInputM = document.getElementById('timerInputM');
  const timerInputS = document.getElementById('timerInputS');
  const timerInputCancel = document.getElementById('timerInputCancel');
  const timerInputSet = document.getElementById('timerInputSet');

  if (timerValText && timerInputPanel) {
    timerValText.addEventListener('click', () => {
      const hrs = Math.floor(defaultTimerTime / 3600);
      const mins = Math.floor((defaultTimerTime % 3600) / 60);
      const secs = defaultTimerTime % 60;
      if (timerInputH) timerInputH.value = hrs;
      if (timerInputM) timerInputM.value = mins;
      if (timerInputS) timerInputS.value = secs;
      
      timerInputPanel.classList.add('active');
    });
  }

  if (timerInputCancel && timerInputPanel) {
    timerInputCancel.addEventListener('click', () => {
      timerInputPanel.classList.remove('active');
    });
  }

  if (timerInputSet && timerInputPanel) {
    timerInputSet.addEventListener('click', () => {
      const hrs = parseInt(timerInputH ? timerInputH.value : '0', 10) || 0;
      const mins = parseInt(timerInputM ? timerInputM.value : '0', 10) || 0;
      const secs = parseInt(timerInputS ? timerInputS.value : '0', 10) || 0;

      const clampedHrs = Math.max(0, hrs);
      const clampedMins = Math.min(59, Math.max(0, mins));
      const clampedSecs = Math.min(59, Math.max(0, secs));

      defaultTimerTime = clampedHrs * 3600 + clampedMins * 60 + clampedSecs;
      resetTimer();
      timerInputPanel.classList.remove('active');
    });
  }

  function renderTimer() {
    if (!timerValText) return;
    const hrs = Math.floor(timerTimeLeft / 3600);
    const mins = Math.floor((timerTimeLeft % 3600) / 60);
    const secs = timerTimeLeft % 60;
    if (hrs > 0) {
      timerValText.textContent = `${padZero(hrs)}:${padZero(mins)}:${padZero(secs)}`;
    } else {
      timerValText.textContent = `${padZero(mins)}:${padZero(secs)}`;
    }
  }

  function startTimer() {
    if (timerInterval) return;
    timerInterval = setInterval(() => {
      if (timerTimeLeft > 0) {
        timerTimeLeft--;
        renderTimer();
      } else {
        clearInterval(timerInterval);
        timerInterval = null;
        updateControlsUI();
        alert("Timer finished!");
      }
    }, 1000);
    updateControlsUI();
  }

  function pauseTimer() {
    if (timerInterval) {
      clearInterval(timerInterval);
      timerInterval = null;
    }
    updateControlsUI();
  }

  function resetTimer() {
    pauseTimer();
    timerTimeLeft = defaultTimerTime;
    renderTimer();
  }

  // 6. Stopwatch Logic
  let stopwatchMs = 0;
  let stopwatchInterval = null;
  let lastStopwatchTick = 0;
  const stopwatchValText = document.getElementById('stopwatchVal');
  const lapList = document.getElementById('lapList');
  let laps = [];

  function renderStopwatch() {
    if (!stopwatchValText) return;
    const mins = Math.floor(stopwatchMs / 60000);
    const secs = Math.floor((stopwatchMs % 60000) / 1000);
    const hunds = Math.floor((stopwatchMs % 1000) / 10);
    stopwatchValText.textContent = `${padZero(mins)}:${padZero(secs)}.${padZero(hunds)}`;
  }

  function startStopwatch() {
    if (stopwatchInterval) return;
    lastStopwatchTick = performance.now();
    stopwatchInterval = setInterval(() => {
      const now = performance.now();
      stopwatchMs += (now - lastStopwatchTick);
      lastStopwatchTick = now;
      renderStopwatch();
    }, 10);
    updateControlsUI();
  }

  function pauseStopwatch() {
    if (stopwatchInterval) {
      clearInterval(stopwatchInterval);
      stopwatchInterval = null;
    }
    updateControlsUI();
  }

  function resetStopwatch() {
    pauseStopwatch();
    stopwatchMs = 0;
    laps = [];
    renderStopwatch();
    renderLaps();
  }

  function recordLap() {
    if (activeMode !== 'stopwatch' && activeMode !== 'lap') return;
    if (stopwatchMs === 0 && laps.length === 0) return;
    
    const lapNum = laps.length + 1;
    const mins = Math.floor(stopwatchMs / 60000);
    const secs = Math.floor((stopwatchMs % 60000) / 1000);
    const hunds = Math.floor((stopwatchMs % 1000) / 10);
    const timeStr = `${padZero(mins)}:${padZero(secs)}.${padZero(hunds)}`;

    laps.unshift({ num: lapNum, time: timeStr });
    renderLaps();

    if (activeMode === 'stopwatch') {
      const lapTab = notch.querySelector('[data-clock-mode="lap"]');
      if (lapTab) lapTab.click();
    }
  }

  function renderLaps() {
    if (!lapList) return;
    if (laps.length === 0) {
      lapList.innerHTML = '<div class="ne-lap-empty">No laps recorded</div>';
      return;
    }

    lapList.innerHTML = laps.map(lap => `
      <div class="ne-lap-item">
        <span class="ne-lap-num">Lap ${lap.num}</span>
        <span class="ne-lap-time">${lap.time}</span>
      </div>
    `).join('');
  }

  if (clockPlayBtn) {
    clockPlayBtn.addEventListener('click', e => {
      e.stopPropagation();
      if (activeMode === 'timer') {
        if (timerInterval) pauseTimer();
        else startTimer();
      } else {
        if (stopwatchInterval) pauseStopwatch();
        else startStopwatch();
      }
    });
  }

  if (clockResetBtn) {
    clockResetBtn.addEventListener('click', e => {
      e.stopPropagation();
      if (activeMode === 'timer') {
        resetTimer();
      } else {
        resetStopwatch();
      }
    });
  }

  if (clockLapBtn) {
    clockLapBtn.addEventListener('click', e => {
      e.stopPropagation();
      recordLap();
    });
  }

  // Initialize display renders
  renderTimer();
  renderStopwatch();
  renderLaps();
  updateControlsUI();

})();


/* ============================================================
   STATS MODULE ENGINE — High fidelity CPU, RAM, GPU trackers
   ============================================================ */
(function initStatsModule() {
  let cpuHistory = Array(20).fill(15);
  let ramHistory = Array(20).fill(45);
  let gpuHistory = Array(20).fill(8);

  const stressState = { cpu: false, ram: false, gpu: false };
  const stressTimeout = { cpu: null, ram: null, gpu: null };
  const stressTargets = { cpu: 0, ram: 0, gpu: 0 };

  function playStatsSound(soundName) {
    try {
      const audio = new Audio(`assets/sounds/${soundName}.wav`);
      audio.volume = 0.35;
      audio.play().catch(err => console.log('Stats audio blocked:', err));
    } catch (e) {
      console.warn('Stats audio error:', e);
    }
  }

  function capitalize(str) {
    return str.charAt(0).toUpperCase() + str.slice(1);
  }

  function triggerStressTest(resource) {
    if (stressState[resource]) return; // already active

    stressState[resource] = true;
    playStatsSound('maximize_004'); // Play stress spike sound

    const cardEl = document.getElementById(`ne${capitalize(resource)}Card`);
    if (cardEl) {
      cardEl.classList.add('stress-active');
    }

    // Target a high resource spike
    stressTargets[resource] = 90 + Math.random() * 8; // 90% - 98%

    if (stressTimeout[resource]) {
      clearTimeout(stressTimeout[resource]);
    }

    // Run stress test for 5 seconds, then decay
    stressTimeout[resource] = setTimeout(() => {
      stressState[resource] = false;
      playStatsSound('close_003'); // Play decay sound
      if (cardEl) {
        cardEl.classList.remove('stress-active');
      }
    }, 5000);
  }

  // Click listeners for each resource card to trigger stress tests
  const cpuCard = document.getElementById('neCpuCard');
  const ramCard = document.getElementById('neRamCard');
  const gpuCard = document.getElementById('neGpuCard');

  if (cpuCard) {
    cpuCard.addEventListener('click', e => {
      e.stopPropagation();
      triggerStressTest('cpu');
    });
  }
  if (ramCard) {
    ramCard.addEventListener('click', e => {
      e.stopPropagation();
      triggerStressTest('ram');
    });
  }
  if (gpuCard) {
    gpuCard.addEventListener('click', e => {
      e.stopPropagation();
      triggerStressTest('gpu');
    });
  }

  // Helper to update SVGs and text value
  function updateResourceUI(resource, history) {
    const svgWidth = 200;
    const svgHeight = 80;

    // Map history points to viewport space
    const points = history.map((val, i) => {
      const x = i * (svgWidth / (history.length - 1));
      // Clamp Y to prevent graph clipping at high/low values
      const y = svgHeight - 6 - (val / 100) * 68;
      return { x: x.toFixed(1), y: y.toFixed(1) };
    });

    const pathData = points.map((p, i) => {
      return (i === 0 ? 'M' : 'L') + ` ${p.x} ${p.y}`;
    }).join(' ');

    const fillData = pathData + ` L ${svgWidth} ${svgHeight} L 0 ${svgHeight} Z`;

    const capRes = capitalize(resource);
    const strokeEl = document.getElementById(`ne${capRes}StrokePath`);
    const fillEl = document.getElementById(`ne${capRes}FillPath`);
    const valEl = document.getElementById(`ne${capRes}Value`);

    if (valEl) {
      valEl.textContent = history[history.length - 1].toFixed(1) + '%';
    }
    if (strokeEl) {
      strokeEl.setAttribute('d', pathData);
    }
    if (fillEl) {
      fillEl.setAttribute('d', fillData);
    }
  }

  // 300ms stats updates
  function tickStats() {
    // 1. CPU values
    let nextCpu;
    if (stressState.cpu) {
      const diff = stressTargets.cpu - cpuHistory[cpuHistory.length - 1];
      nextCpu = cpuHistory[cpuHistory.length - 1] + diff * 0.35 + (Math.random() - 0.5) * 8;
      nextCpu = Math.min(100, Math.max(82, nextCpu));
    } else {
      const last = cpuHistory[cpuHistory.length - 1];
      const target = 10 + Math.random() * 12; // baseline target 10-22%
      nextCpu = last + (target - last) * 0.12 + (Math.random() - 0.5) * 4;
      nextCpu = Math.min(50, Math.max(3, nextCpu));
    }
    cpuHistory.shift();
    cpuHistory.push(nextCpu);

    // 2. Memory/RAM values
    let nextRam;
    if (stressState.ram) {
      const diff = stressTargets.ram - ramHistory[ramHistory.length - 1];
      nextRam = ramHistory[ramHistory.length - 1] + diff * 0.25 + (Math.random() - 0.5) * 3;
      nextRam = Math.min(99, Math.max(75, nextRam));
    } else {
      const last = ramHistory[ramHistory.length - 1];
      const target = 44.5 + Math.random() * 1.5; // baseline target 44.5-46%
      nextRam = last + (target - last) * 0.08 + (Math.random() - 0.5) * 0.6;
      nextRam = Math.min(52, Math.max(40, nextRam));
    }
    ramHistory.shift();
    ramHistory.push(nextRam);

    // 3. GPU values
    let nextGpu;
    if (stressState.gpu) {
      const diff = stressTargets.gpu - gpuHistory[gpuHistory.length - 1];
      nextGpu = gpuHistory[gpuHistory.length - 1] + diff * 0.3 + (Math.random() - 0.5) * 9;
      nextGpu = Math.min(100, Math.max(78, nextGpu));
    } else {
      const last = gpuHistory[gpuHistory.length - 1];
      const target = 4 + Math.random() * 8; // baseline target 4-12%
      nextGpu = last + (target - last) * 0.15 + (Math.random() - 0.5) * 3;
      nextGpu = Math.min(45, Math.max(1, nextGpu));
    }
    gpuHistory.shift();
    gpuHistory.push(nextGpu);

    // Render updates
    updateResourceUI('cpu', cpuHistory);
    updateResourceUI('ram', ramHistory);
    updateResourceUI('gpu', gpuHistory);
  }

  // Start tick
  tickStats();
  setInterval(tickStats, 300);
})();


/* ============================================================
   LEGAL MODAL CONTROLLER
   ============================================================ */
(function initLegalModal() {
  const modal = document.getElementById('legalModal');
  const title = document.getElementById('legalModalTitle');
  const body = document.getElementById('legalModalBody');
  const closeBtn = document.getElementById('legalModalClose');

  const links = {
    privacy: {
      title: 'Privacy Policy',
      html: `
        <div class="legal-content">
          <p><strong>Effective Date: June 5, 2026</strong></p>
          <br>
          <p>Your privacy is extremely important to us. This Privacy Policy explains how Raven Notch handles data when you use our application and website.</p>
          
          <h4>1. Zero Personal Data Collection</h4>
          <p>We do not collect, store, transmit, or share any of your personal data. Raven Notch operates entirely as a local desktop utility. We do not run any telemetry services or remote tracking scripts.</p>
          
          <h4>2. Google Integration (Calendar Module)</h4>
          <p>If you choose to connect your Google Account to synchronize your calendar agendas:</p>
          <ul>
            <li>All OAuth authentication tokens and calendar event details are fetched directly from Google's APIs and stored <strong>only locally</strong> on your secure computer.</li>
            <li>Your calendar details are processed locally to render the widgets and never leave your machine.</li>
            <li>We never transmit your calendar data to our servers or any third parties.</li>
          </ul>
          
          <h4>3. Compliance with Google API Services User Data Policy</h4>
          <p>Raven Notch's use and transfer of information received from Google APIs to any other app will adhere to the <a href="https://developers.google.com/terms/api-services-user-data-policy" target="_blank" style="color: #ffffff; text-decoration: underline;">Google API Services User Data Policy</a>, including the Limited Use requirements.</p>

          <h4>4. Local Security</h4>
          <p>All application settings and cached module views are stored in your local application directory (AppData). We recommend maintaining basic OS security controls to protect your local data.</p>
          
          <h4>5. Changes to This Policy</h4>
          <p>We may update our Privacy Policy from time to time. Any changes will be posted on this page with an updated effective date.</p>
          
          <h4>6. Contact Us</h4>
          <p>If you have any questions about this Privacy Policy, please contact us at <strong>connect@ravennotch.me</strong>.</p>
        </div>
      `
    },
    terms: {
      title: 'Terms of Service',
      html: `
        <div class="legal-content">
          <p><strong>Effective Date: June 5, 2026</strong></p>
          <br>
          <p>By using the Raven Notch website and desktop application, you agree to comply with and be bound by the following Terms of Service.</p>
          
          <h4>1. License Agreement</h4>
          <p>Subject to these terms, Raven Notch grants you a limited, non-exclusive, non-transferable, and revocable license to use the software on compatible Windows operating systems for your personal or professional use.</p>
          
          <h4>2. Trial Period & Licensing</h4>
          <p>We provide a fully-featured free trial period. Following the expiration of the trial period, a valid paid license key is required to continue utilizing the premium modules. License keys are for individual use and must not be distributed or shared.</p>
          
          <h4>3. Refund Policy</h4>
          <p>Because we offer a fully-functional free trial period to evaluate the software and premium features prior to purchase, all transactions for paid license keys are final and non-refundable.</p>
          
          <h4>4. Acceptable Use</h4>
          <p>You agree not to decompile, reverse-engineer, modify, or attempt to extract the source code of the Raven Notch desktop application, except as permitted under applicable local law.</p>
          
          <h4>5. Google API Usage</h4>
          <p>The Calendar widget interacts with Google API services to display your agenda. You agree to use this integration in accordance with Google's Terms of Service and API policies. Any abuse of Google API services through our client is strictly prohibited.</p>
          
          <h4>6. Disclaimer of Warranties</h4>
          <p>The software and services are provided "as is" and "as available" without any warranties of any kind, express or implied. Raven Notch does not guarantee that the application will be completely free of errors or interruptions.</p>
          
          <h4>7. Limitation of Liability</h4>
          <p>In no event shall Raven Notch be liable for any direct, indirect, incidental, special, or consequential damages arising out of your use or inability to use the software, even if advised of the possibility of such damages.</p>
          
          <h4>8. Governing Law</h4>
          <p>These terms shall be governed by and construed in accordance with the laws of your local jurisdiction, without regard to conflict of law principles.</p>
        </div>
      `
    },
    changelog: {
      title: 'System Changelog',
      html: `
        <div class="changelog-container">
          <div class="changelog-release">
            <div class="changelog-release-header">
              <span class="changelog-version">v0.1.0</span>
              <span class="changelog-date">June 2026</span>
              <span class="changelog-badge">Initial Release</span>
            </div>
            <p class="changelog-release-desc" style="margin-top: 8px;">Welcome to the first version of Raven Notch! All widgets and overlays are now active. Subsequent updates and release notes will be posted in this panel as they become available.</p>
          </div>
        </div>
      `
    }
  };

  if (!modal || !title || !body) return;

  const openModal = (type) => {
    const data = links[type];
    if (!data) return;

    title.textContent = data.title;
    body.innerHTML = data.html;
    modal.classList.add('active');
    document.body.style.overflow = 'hidden';
  };

  const closeModal = () => {
    modal.classList.remove('active');
    const loadingScreen = document.getElementById('loadingScreen');
    if (!loadingScreen || loadingScreen.classList.contains('hidden')) {
      document.body.style.overflow = '';
    }
    const activeHash = window.location.hash;
    if (activeHash === '#changelog' || activeHash === '#privacy' || activeHash === '#terms') {
      history.replaceState(null, null, ' ');
    }
  };

  const navPrivacy = document.getElementById('navPrivacy');
  const navTerms = document.getElementById('navTerms');
  const navChangelog = document.getElementById('navChangelog');
  const mobilePrivacy = document.getElementById('mobilePrivacy');
  const mobileTerms = document.getElementById('mobileTerms');
  const mobileChangelog = document.getElementById('mobileChangelog');

  if (navPrivacy) navPrivacy.addEventListener('click', (e) => { e.preventDefault(); openModal('privacy'); });
  if (navTerms) navTerms.addEventListener('click', (e) => { e.preventDefault(); openModal('terms'); });
  if (navChangelog) navChangelog.addEventListener('click', (e) => { e.preventDefault(); openModal('changelog'); });
  
  if (mobilePrivacy) mobilePrivacy.addEventListener('click', (e) => { 
    e.preventDefault(); 
    const hamburger = document.getElementById('navHamburger');
    const mobileMenu = document.getElementById('navMobileMenu');
    if (hamburger && mobileMenu) {
      hamburger.classList.remove('open');
      mobileMenu.classList.remove('open');
    }
    openModal('privacy'); 
  });
  
  if (mobileTerms) mobileTerms.addEventListener('click', (e) => { 
    e.preventDefault(); 
    const hamburger = document.getElementById('navHamburger');
    const mobileMenu = document.getElementById('navMobileMenu');
    if (hamburger && mobileMenu) {
      hamburger.classList.remove('open');
      mobileMenu.classList.remove('open');
    }
    openModal('terms'); 
  });

  if (mobileChangelog) mobileChangelog.addEventListener('click', (e) => { 
    e.preventDefault(); 
    const hamburger = document.getElementById('navHamburger');
    const mobileMenu = document.getElementById('navMobileMenu');
    if (hamburger && mobileMenu) {
      hamburger.classList.remove('open');
      mobileMenu.classList.remove('open');
    }
    openModal('changelog'); 
  });

  if (closeBtn) closeBtn.addEventListener('click', closeModal);
  modal.addEventListener('click', (e) => {
    if (e.target === modal) closeModal();
  });
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modal.classList.contains('active')) {
      closeModal();
    }
  });

  const checkHash = () => {
    if (window.location.hash === '#changelog') {
      openModal('changelog');
    } else if (window.location.hash === '#privacy') {
      openModal('privacy');
    } else if (window.location.hash === '#terms') {
      openModal('terms');
    }
  };
  window.addEventListener('hashchange', checkHash);
  setTimeout(checkHash, 600);
})();


/* ============================================================
   LOGIN MODAL
   ============================================================ */
(function initLoginModal() {
  const modal = document.getElementById('loginModal');
  const closeBtn = document.getElementById('loginModalClose');
  const openBtns = [
    document.getElementById('navLogin'),
    document.getElementById('mobileLogin')
  ].filter(Boolean);
  const googleBtn = document.getElementById('loginGoogleBtn');
  const title = document.getElementById('loginModalTitle');
  const subtitle = document.getElementById('loginModalSubtitle');
  const loginForm = document.getElementById('loginAccountForm');
  const signupForm = document.getElementById('signupAccountForm');
  const switchBtn = document.getElementById('loginSwitchBtn');
  const errorText = document.getElementById('loginErrorText');
  const usernameInput = signupForm?.querySelector('input[name="username"]');
  const accountPopover = document.getElementById('accountPopover');
  const accountPopoverClose = document.getElementById('accountPopoverClose');
  const accountAvatarLarge = document.getElementById('accountAvatarLarge');
  const accountName = document.getElementById('accountName');
  const accountUsername = document.getElementById('accountUsername');
  const accountPlanStatus = document.getElementById('accountPlanStatus');
  const accountPlanText = document.getElementById('accountPlanText');
  const accountEditBtn = document.getElementById('accountEditBtn');
  const accountLogoutBtn = document.getElementById('accountLogoutBtn');
  const accountCardInner = document.getElementById('accountCardInner');
  const accountEditForm = document.getElementById('accountEditForm');
  const accountEditError = document.getElementById('accountEditError');
  const accountPhotoPicker = document.getElementById('accountPhotoPicker');
  const accountPhotoInput = document.getElementById('accountPhotoInput');
  const accountPhotoPreview = document.getElementById('accountPhotoPreview');
  const usernameRule = /^[a-z0-9._-]{3,24}$/;
  let mode = 'login';
  let currentAccount = null;
  let currentPurchase = null;

  const applyAccountState = (account) => {
    if (!account?.authenticated || !account.user) return;
    currentAccount = account.user;
    currentPurchase = account.purchase || null;
    const label = account.user.name || account.user.email || 'My account';
    const initial = (label.trim()[0] || 'R').toUpperCase();
    const picture = highResAvatar(account.user.picture, 160);
    const avatarMarkup = picture
      ? `<img class="nav-account-avatar" src="${picture}" alt="${label}">`
      : `<span class="nav-account-fallback">${initial}</span>`;
    openBtns.forEach((btn) => {
      btn.innerHTML = avatarMarkup;
      btn.classList.add('signed-in');
      btn.title = account.user.email || label;
      btn.setAttribute('aria-label', 'Open account menu');
    });
  };
  window.applyAccountState = applyAccountState;

  if (!modal || !openBtns.length) return;

  const displayUsername = (user) => {
    const fallback = String(user?.email || 'raven').split('@')[0] || 'raven';
    return `@${String(user?.username || fallback).replace(/^@+/, '')}`;
  };

  const highResAvatar = (picture, size = 420) => {
    const url = String(picture || '').trim();
    if (!url || url.startsWith('data:')) return url;
    return url
      .replace(/=s\d+(-c)?/i, `=s${size}-c`)
      .replace(/\/s\d+(-c)?\//i, `/s${size}-c/`);
  };

  const renderLargeAvatar = (user) => {
    if (!accountAvatarLarge) return;
    const label = user?.name || user?.email || 'Raven User';
    const picture = highResAvatar(user?.picture, 520);
    if (picture) {
      accountAvatarLarge.innerHTML = `<img src="${picture}" alt="${label}">`;
    } else {
      accountAvatarLarge.textContent = (label.trim()[0] || 'R').toUpperCase();
    }
  };

  const renderPhotoPreview = (picture, label = 'Raven User') => {
    if (!accountPhotoPreview) return;
    const src = highResAvatar(picture, 360);
    if (src) {
      accountPhotoPreview.innerHTML = `<img src="${src}" alt="${label}">`;
    } else {
      accountPhotoPreview.textContent = (label.trim()[0] || 'R').toUpperCase();
    }
  };

  const renderPurchaseStatus = () => {
    if (!accountPlanStatus || !accountPlanText) return;
    const active = currentPurchase?.status === 'active';
    accountPlanStatus.classList.toggle('active', active);
    accountPlanText.textContent = active
      ? (currentPurchase?.plan === 'lifetime' ? 'Lifetime active' : 'Plan active')
      : 'Not purchased';
  };

  const compressProfileImage = (file) => new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error('Unable to read image'));
    reader.onload = () => {
      const img = new Image();
      img.onerror = () => reject(new Error('Choose a valid image file'));
      img.onload = () => {
        const size = 360;
        const canvas = document.createElement('canvas');
        const ctx = canvas.getContext('2d');
        const sourceSize = Math.min(img.width, img.height);
        const sx = Math.max(0, (img.width - sourceSize) / 2);
        const sy = Math.max(0, (img.height - sourceSize) / 2);
        canvas.width = size;
        canvas.height = size;
        ctx.drawImage(img, sx, sy, sourceSize, sourceSize, 0, 0, size, size);
        resolve(canvas.toDataURL('image/jpeg', 0.82));
      };
      img.src = reader.result;
    };
    reader.readAsDataURL(file);
  });

  const openAccountMenu = () => {
    if (!currentAccount || !accountPopover) return;
    if (accountName) accountName.textContent = currentAccount.name || 'Raven User';
    if (accountUsername) accountUsername.textContent = displayUsername(currentAccount);
    renderLargeAvatar(currentAccount);
    renderPurchaseStatus();
    accountPopover.querySelector('.account-card')?.classList.remove('editing');
    accountPopover.classList.add('active');
    accountPopover.setAttribute('aria-hidden', 'false');
    document.body.style.overflow = 'hidden';
  };

  const closeAccountMenu = () => {
    if (!accountPopover) return;
    accountPopover.classList.remove('active');
    accountPopover.setAttribute('aria-hidden', 'true');
    const loadingScreen = document.getElementById('loadingScreen');
    if (!modal.classList.contains('active') && (!loadingScreen || loadingScreen.classList.contains('hidden'))) {
      document.body.style.overflow = '';
    }
  };

  const resetAccountState = () => {
    currentAccount = null;
    currentPurchase = null;
    openBtns.forEach((btn) => {
      btn.textContent = 'Log in';
      btn.classList.remove('signed-in');
      btn.removeAttribute('title');
      btn.setAttribute('aria-label', 'Log in');
    });
  };

  const open = () => {
    modal.classList.add('active');
    modal.setAttribute('aria-hidden', 'false');
    document.body.style.overflow = 'hidden';
  };

  const setMode = (nextMode) => {
    mode = nextMode;
    const isSignup = mode === 'signup';
    if (title) title.textContent = isSignup ? 'Create your account' : 'Log in to Raven Notch';
    if (subtitle) subtitle.textContent = isSignup ? 'Create a Raven account before purchasing or activating the app.' : 'Use your Raven account to manage purchases and activate the app.';
    loginForm?.classList.toggle('active', !isSignup);
    signupForm?.classList.toggle('active', isSignup);
    if (switchBtn) switchBtn.textContent = isSignup ? 'I already have an account →' : "Don't have an account? Sign up →";
    if (errorText) errorText.textContent = '';
  };

  const submitAccountForm = async (form, endpoint) => {
    if (!form) return;
    const button = form.querySelector('button[type="submit"]');
    const fields = Array.from(form.querySelectorAll('input'));
    fields.forEach((field) => field.classList.remove('invalid'));
    if (errorText) errorText.textContent = '';
    const body = Object.fromEntries(new FormData(form).entries());
    if (endpoint.includes('/signup')) {
      body.username = String(body.username || '').trim();
      const usernameField = form.querySelector('input[name="username"]');
      if (!usernameRule.test(body.username)) {
        usernameField?.classList.add('invalid');
        if (errorText) {
          errorText.textContent = 'Use 3-24 characters: lowercase letters, numbers, dot, underscore, or hyphen.';
        }
        return;
      }
    }
    if (button) button.disabled = true;
    try {
      const response = await fetch(endpoint, {
        method: 'POST',
        credentials: 'include',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(body),
      });
      const data = await response.json().catch(() => ({}));
      if (!response.ok) throw new Error(data.error || 'Authentication failed');
      applyAccountState(data);
      close();
      if (typeof window.showToast === 'function') {
        window.showToast(mode === 'signup' ? 'Raven account created' : 'Signed in to Raven Notch');
      }
    } catch (error) {
      if (errorText) errorText.textContent = error.message || 'Authentication failed';
    } finally {
      if (button) button.disabled = false;
    }
  };

  const close = () => {
    modal.classList.remove('active');
    modal.setAttribute('aria-hidden', 'true');
    const loadingScreen = document.getElementById('loadingScreen');
    if (!loadingScreen || loadingScreen.classList.contains('hidden')) {
      document.body.style.overflow = '';
    }
  };

  openBtns.forEach((btn) => {
    btn.addEventListener('click', (event) => {
      event.preventDefault();
      if (currentAccount) {
        openAccountMenu();
        return;
      }
      const hamburger = document.getElementById('navHamburger');
      const mobileMenu = document.getElementById('navMobileMenu');
      if (hamburger && mobileMenu) {
        hamburger.classList.remove('open');
        mobileMenu.classList.remove('open');
      }
      open();
    });
  });

  if (closeBtn) closeBtn.addEventListener('click', close);
  if (accountPopoverClose) accountPopoverClose.addEventListener('click', closeAccountMenu);
  if (accountPopover) {
    accountPopover.addEventListener('click', (event) => {
      if (event.target === accountPopover) closeAccountMenu();
    });
  }
  if (accountEditBtn) {
    accountEditBtn.addEventListener('click', () => {
      if (!currentAccount || !accountEditForm) return;
      accountEditForm.elements.name.value = currentAccount.name || '';
      accountEditForm.elements.username.value = String(currentAccount.username || currentAccount.email?.split('@')[0] || '').toLowerCase();
      accountEditForm.elements.picture.value = currentAccount.picture || '';
      accountEditForm.elements.password.value = '';
      renderPhotoPreview(currentAccount.picture, currentAccount.name || currentAccount.email || 'Raven User');
      if (accountEditError) accountEditError.textContent = '';
      accountPopover?.querySelector('.account-card')?.classList.add('editing');
    });
  }
  if (accountPhotoPicker && accountPhotoInput) {
    accountPhotoPicker.addEventListener('click', () => accountPhotoInput.click());
    accountPhotoInput.addEventListener('change', async () => {
      const file = accountPhotoInput.files?.[0];
      if (!file || !accountEditForm) return;
      if (!file.type.startsWith('image/')) {
        if (accountEditError) accountEditError.textContent = 'Choose an image file.';
        return;
      }
      try {
        const dataUrl = await compressProfileImage(file);
        accountEditForm.elements.picture.value = dataUrl;
        renderPhotoPreview(dataUrl, currentAccount?.name || 'Raven User');
        if (accountEditError) accountEditError.textContent = '';
      } catch (error) {
        if (accountEditError) accountEditError.textContent = error.message || 'Unable to use that image.';
      }
    });
  }
  if (accountEditForm) {
    accountEditForm.addEventListener('submit', async (event) => {
      event.preventDefault();
      const submit = accountEditForm.querySelector('button[type="submit"]');
      const body = Object.fromEntries(new FormData(accountEditForm).entries());
      body.username = String(body.username || '').trim().toLowerCase();
      if (body.username && !usernameRule.test(body.username)) {
        if (accountEditError) accountEditError.textContent = 'Username cannot contain spaces. Use lowercase letters, numbers, dot, underscore, or hyphen.';
        return;
      }
      if (accountEditError) accountEditError.textContent = '';
      if (submit) submit.disabled = true;
      try {
        const response = await fetch('/api/auth/profile', {
          method: 'POST',
          credentials: 'include',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify(body),
        });
        const data = await response.json().catch(() => ({}));
        if (!response.ok) throw new Error(data.error || 'Unable to update profile');
        applyAccountState(data);
        if (accountName) accountName.textContent = currentAccount.name || 'Raven User';
        if (accountUsername) accountUsername.textContent = displayUsername(currentAccount);
        renderLargeAvatar(currentAccount);
        accountPopover?.querySelector('.account-card')?.classList.remove('editing');
        if (typeof window.showToast === 'function') window.showToast('Profile updated');
      } catch (error) {
        if (accountEditError) accountEditError.textContent = error.message || 'Unable to update profile';
      } finally {
        if (submit) submit.disabled = false;
      }
    });
  }
  if (accountLogoutBtn) {
    accountLogoutBtn.addEventListener('click', async () => {
      accountLogoutBtn.disabled = true;
      try {
        await fetch('/api/auth/logout', { method: 'POST', credentials: 'include', cache: 'no-store' });
        resetAccountState();
        closeAccountMenu();
        if (typeof window.showToast === 'function') window.showToast('Logged out');
      } finally {
        accountLogoutBtn.disabled = false;
      }
    });
  }
  modal.addEventListener('click', (event) => {
    if (event.target === modal) close();
  });
  window.addEventListener('keydown', (event) => {
    if (event.key === 'Escape' && modal.classList.contains('active')) close();
    if (event.key === 'Escape' && accountPopover?.classList.contains('active')) closeAccountMenu();
  });

  if (googleBtn) {
    googleBtn.addEventListener('click', () => {
      googleBtn.classList.add('loading');
    });
  }

  if (switchBtn) {
    switchBtn.addEventListener('click', () => setMode(mode === 'signup' ? 'login' : 'signup'));
  }
  if (usernameInput) {
    usernameInput.addEventListener('input', () => {
      const value = usernameInput.value.toLowerCase();
      if (usernameInput.value !== value) usernameInput.value = value;
      const hasValue = value.length > 0;
      const isValid = usernameRule.test(value);
      usernameInput.classList.toggle('invalid', hasValue && !isValid);
      if (errorText && hasValue && !isValid) {
        errorText.textContent = value.includes(' ')
          ? 'Username cannot contain spaces. Use lowercase letters, numbers, dot, underscore, or hyphen.'
          : 'Use 3-24 characters: lowercase letters, numbers, dot, underscore, or hyphen.';
      } else if (errorText?.textContent?.includes('lowercase letters') || errorText?.textContent?.includes('spaces')) {
        errorText.textContent = '';
      }
    });
  }
  if (loginForm) {
    loginForm.addEventListener('submit', (event) => {
      event.preventDefault();
      submitAccountForm(loginForm, '/api/auth/login');
    });
  }
  if (signupForm) {
    signupForm.addEventListener('submit', (event) => {
      event.preventDefault();
      submitAccountForm(signupForm, '/api/auth/signup');
    });
  }

  const loginResult = new URLSearchParams(window.location.search).get('login');
  if (loginResult === 'success') {
    if (typeof window.showToast === 'function') {
      window.showToast('Signed in to Raven Notch');
    }
    history.replaceState(null, '', window.location.pathname + window.location.hash);
  } else if (loginResult === 'account_required') {
    setMode('signup');
    open();
    if (errorText) errorText.textContent = 'Create your Raven account first, then sign in with that same account.';
    history.replaceState(null, '', window.location.pathname + window.location.hash);
  }

  if (location.protocol === 'https:' || location.hostname === 'localhost') {
    fetch('/api/auth/me', { credentials: 'include', cache: 'no-store' })
      .then((response) => response.json())
      .then(applyAccountState)
      .catch(() => {});
  }
})();


/* ============================================================
   FEEDBACK MODAL & DYNAMIC REVIEWS
   ============================================================ */
(function initFeedbackModal() {
  const modal = document.getElementById('feedbackModal');
  const openBtns = [
    document.getElementById('navFeedback'),
    document.getElementById('mobileFeedback')
  ];
  const closeBtn = document.getElementById('feedbackModalClose');
  const form = document.getElementById('feedbackForm');

  const fbUsername = document.getElementById('fbUsername');
  const fbQuote = document.getElementById('fbQuote');
  const fbCharCount = document.getElementById('fbCharCount');
  const starSelector = document.getElementById('starSelector');
  const starBtns = starSelector ? starSelector.querySelectorAll('.star-btn') : [];
  const ratingInput = document.getElementById('fbRating');

  if (!modal || !form) return;

  const openModal = () => {
    modal.classList.add('active');
    document.body.style.overflow = 'hidden';
    
    // Reset form states
    form.reset();
    if (fbCharCount) fbCharCount.textContent = '0';
    if (ratingInput) ratingInput.value = '5';
    updateStars(5);
  };

  const closeModal = () => {
    modal.classList.remove('active');
    const loadingScreen = document.getElementById('loadingScreen');
    if (!loadingScreen || loadingScreen.classList.contains('hidden')) {
      document.body.style.overflow = '';
    }
  };

  // Bind nav buttons
  openBtns.forEach(btn => {
    if (!btn) return;
    btn.addEventListener('click', (e) => {
      e.preventDefault();
      const hamburger = document.getElementById('navHamburger');
      const mobileMenu = document.getElementById('navMobileMenu');
      if (hamburger && mobileMenu) {
        hamburger.classList.remove('open');
        mobileMenu.classList.remove('open');
      }
      openModal();
    });
  });

  if (closeBtn) closeBtn.addEventListener('click', closeModal);
  modal.addEventListener('click', (e) => {
    if (e.target === modal) closeModal();
  });
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && modal.classList.contains('active')) {
      closeModal();
    }
  });

  // Char count updates
  if (fbQuote && fbCharCount) {
    fbQuote.addEventListener('input', () => {
      fbCharCount.textContent = fbQuote.value.length;
    });
  }

  // Star selector logic
  function updateStars(val) {
    starBtns.forEach(btn => {
      const v = parseInt(btn.dataset.value);
      btn.classList.toggle('active', v <= val);
    });
  }

  if (starSelector) {
    starBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        const val = parseInt(btn.dataset.value);
        if (ratingInput) ratingInput.value = val;
        updateStars(val);
      });

      btn.addEventListener('mouseenter', () => {
        const val = parseInt(btn.dataset.value);
        starBtns.forEach(b => {
          const v = parseInt(b.dataset.value);
          b.classList.toggle('active', v <= val);
        });
      });
    });

    starSelector.addEventListener('mouseleave', () => {
      const val = ratingInput ? parseInt(ratingInput.value) : 5;
      updateStars(val);
    });
  }

  // Form submission & testimonial injection
  form.addEventListener('submit', (e) => {
    e.preventDefault();

    let username = fbUsername ? fbUsername.value.trim() : '';
    const quote = fbQuote ? fbQuote.value.trim() : '';
    const rating = ratingInput ? parseInt(ratingInput.value) : 5;

    if (!username || !quote) return;

    // Normalize username handle
    if (!username.startsWith('@')) {
      username = '@' + username;
    }

    // Get initials for avatar (e.g. "@dev" -> "DE", "raunak" -> "RA")
    let cleanUser = username.replace('@', '');
    let initials = 'UR';
    if (cleanUser.length >= 2) {
      initials = cleanUser.substring(0, 2).toUpperCase();
    } else if (cleanUser.length === 1) {
      initials = cleanUser.substring(0, 1).toUpperCase() + 'U';
    }

    // Build star rating text
    let starsStr = '';
    for (let i = 0; i < rating; i++) {
      starsStr += '⭐';
    }

    // Create testimonial card element only if it has a 5-star rating
    if (rating === 5) {
      const carousel = document.getElementById('testimonialsCarousel');
      if (carousel) {
        // Remove placeholder card if present
        const placeholder = carousel.querySelector('.placeholder-card');
        if (placeholder) {
          placeholder.remove();
        }

        const card = document.createElement('div');
        card.className = 'testimonial-card';
        card.innerHTML = `
          <div class="card-dot-grid" aria-hidden="true"></div>
          <p class="testimonial-quote">"${quote}"</p>
          <div class="testimonial-author">
            <div class="author-avatar">${initials}</div>
            <div class="author-info">
              <h4 class="author-name">${username}</h4>
              <p class="author-title">Power User (${starsStr})</p>
            </div>
          </div>
        `;

        // Prepend to the carousel so it shows up as the first item
        carousel.insertBefore(card, carousel.firstChild);

        // Refresh carousel slide count and dots
        if (typeof window.refreshTestimonialsCarousel === 'function') {
          window.refreshTestimonialsCarousel();
        }

        // Scroll testimonials section into view smoothly
        const testimonialsSection = document.getElementById('testimonials');
        if (testimonialsSection) {
          testimonialsSection.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }
      }
    }

    closeModal();

    // Show success notification
    if (typeof window.showToast === 'function') {
      window.showToast('💖 Thank you! Your review is now live in the testimonials.');
    } else {
      alert('💖 Thank you! Your review is now live in the testimonials.');
    }
  });

  // Open feedback modal if URL hash is #feedback on page load or hash change
  const checkHash = () => {
    if (window.location.hash === '#feedback') {
      openModal();
    }
  };
  window.addEventListener('hashchange', checkHash);
  setTimeout(checkHash, 600);
})();



/* ============================================================
   GEOLOCATION DYNAMIC PRICING CONTROLLER
   ============================================================ */
(function initDynamicPricing() {
  fetch('/api/pricing')
    .then(res => res.json())
    .then(data => {
      if (data.priceText) {
        // Update all purchase price elements (buttons)
        document.querySelectorAll('.purchase-price').forEach(el => {
          el.textContent = data.priceText;
        });
        // Update price placeholders in description text paragraphs
        document.querySelectorAll('.purchase-price-text').forEach(el => {
          el.textContent = data.priceText;
        });
      }
    })
    .catch(err => console.error('Failed to load dynamic pricing:', err));
})();



/* ============================================================
   ACTIVE NAV LINK SPY CONTROLLER
   ============================================================ */
(function initNavActiveSpy() {
  const navLinks = document.querySelectorAll('.nav-links .nav-link');
  const mobileLinks = document.querySelectorAll('.nav-mobile-links .nav-link');

  const removeActive = () => {
    navLinks.forEach(link => link.classList.remove('active'));
    mobileLinks.forEach(link => link.classList.remove('active'));
  };

  const addActive = (href) => {
    navLinks.forEach(link => {
      if (link.getAttribute('href') === href) {
        link.classList.add('active');
      }
    });
    mobileLinks.forEach(link => {
      if (link.getAttribute('href') === href) {
        link.classList.add('active');
      }
    });
  };

  // intersection observer to detect scroll positions
  const observerOptions = {
    root: null,
    rootMargin: '-20% 0px -50% 0px',
    threshold: 0.1
  };

  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        const id = entry.target.id;
        removeActive();
        if (id === 'hero') {
          addActive('#');
        } else {
          addActive(`#${id}`);
        }
      }
    });
  }, observerOptions);

  const heroSection = document.getElementById('hero');
  const featuresSection = document.getElementById('features');
  const downloadSection = document.getElementById('download');

  if (heroSection) observer.observe(heroSection);
  if (featuresSection) observer.observe(featuresSection);
  if (downloadSection) observer.observe(downloadSection);

  // handle click events
  const handleLinkClick = (e) => {
    const href = e.currentTarget.getAttribute('href');
    // For local anchors, update active state immediately
    if (href && href.startsWith('#')) {
      removeActive();
      e.currentTarget.classList.add('active');
    }
  };

  navLinks.forEach(link => {
    link.addEventListener('click', handleLinkClick);
  });
  mobileLinks.forEach(link => {
    link.addEventListener('click', handleLinkClick);
  });
})();

