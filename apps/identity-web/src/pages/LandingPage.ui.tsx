import { Check } from 'lucide-react';

export function CheckList({ items, tone = 'light' }: { items: string[]; tone?: 'light' | 'dark' }) {
  const textClass = tone === 'dark' ? 'text-teal-50' : 'text-zinc-700';
  const iconClass = tone === 'dark' ? 'text-white' : 'text-teal-700';

  return (
    <ul className="space-y-3">
      {items.map((item) => (
        <li className={`flex gap-3 text-sm leading-6 ${textClass}`} key={item}>
          <Check className={`mt-1 size-4 ${iconClass}`} />
          <span>{item}</span>
        </li>
      ))}
    </ul>
  );
}
