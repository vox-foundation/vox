import React from 'react';

export function Backdrop() {
  return (
    <>
      <div className="pointer-events-none fixed inset-0 -z-10 bg-[#F6F1E6]" />
      <div
        className="pointer-events-none fixed inset-0 -z-10 opacity-[0.94]"
        style={{
          backgroundImage: 'url(/assets/romanesque/carrara-page-watermark-bg.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
      />
      <div className="pointer-events-none fixed inset-0 -z-10 [background:radial-gradient(950px_540px_at_18%_-8%,rgba(255,252,245,0.55),transparent_62%),radial-gradient(900px_520px_at_84%_108%,rgba(212,175,55,0.16),transparent_62%)]" />
    </>
  );
}
