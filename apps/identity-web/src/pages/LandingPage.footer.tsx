export function LandingPageFooter() {
  return (
    <footer className="border-t border-zinc-200 px-6 py-9 text-sm text-zinc-600 lg:px-10" id="docs">
      <div className="mx-auto grid max-w-7xl gap-10 md:grid-cols-[1.2fr_1fr_1fr_1fr_1fr]">
        <div>
          <div className="text-2xl font-semibold text-zinc-950">nvbes</div>
          <p className="mt-3 max-w-40 leading-6">
            Identity and files for teams that ship securely.
          </p>
        </div>
        {['Platform', 'Security', 'Docs', 'Company'].map((group) => (
          <div key={group}>
            <h3 className="font-semibold text-zinc-950">{group}</h3>
            <div className="mt-3 space-y-2">
              {['Overview', 'Guides', 'API reference', 'Status'].map((item) => (
                <a className="block" href="#platform" key={`${group}-${item}`}>
                  {item}
                </a>
              ))}
            </div>
          </div>
        ))}
      </div>
    </footer>
  );
}
