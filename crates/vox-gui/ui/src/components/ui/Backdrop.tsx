import React from 'react';

export function Backdrop() {
  return (
    <>
      <div className="vox-backdrop-base pointer-events-none fixed inset-0 -z-10 bg-bg-base" />
      <div
        className="vox-backdrop-marble pointer-events-none fixed inset-0 -z-10 opacity-[0.94]"
        style={{
          backgroundImage: 'url(/assets/romanesque/carrara-seamless-stone.jpg)',
          backgroundSize: 'cover',
          backgroundPosition: 'center',
        }}
      />
      <div className="vox-backdrop-glow pointer-events-none fixed inset-0 -z-10 [background:radial-gradient(950px_540px_at_18%_-8%,rgba(255,252,245,0.55),transparent_62%),radial-gradient(900px_520px_at_84%_108%,rgba(212,175,55,0.16),transparent_62%)]" />
    </>
  );
}
