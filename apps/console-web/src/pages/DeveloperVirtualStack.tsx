import { useVirtualizer } from '@tanstack/react-virtual';
import type { ReactNode } from 'react';
import { useRef } from 'react';

type DeveloperVirtualStackProps<T> = {
  items: T[];
  threshold?: number;
  estimateSize: number;
  className?: string;
  itemClassName?: string;
  getKey: (item: T) => string;
  renderItem: (item: T) => ReactNode;
};

export function DeveloperVirtualStack<T>({
  items,
  threshold = 50,
  estimateSize,
  className,
  itemClassName,
  getKey,
  renderItem,
}: DeveloperVirtualStackProps<T>) {
  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => estimateSize,
    overscan: 8,
  });

  if (items.length <= threshold) {
    return <div className={className}>{items.map((item) => renderItem(item))}</div>;
  }

  return (
    <div ref={parentRef} className={className}>
      <div className="relative w-full" style={{ height: `${virtualizer.getTotalSize()}px` }}>
        {virtualizer.getVirtualItems().map((virtualItem) => {
          const item = items[virtualItem.index];
          if (!item) {
            return null;
          }
          return (
            <div
              key={getKey(item)}
              className={itemClassName}
              data-index={virtualItem.index}
              ref={virtualizer.measureElement}
              style={{
                position: 'absolute',
                top: 0,
                left: 0,
                width: '100%',
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              {renderItem(item)}
            </div>
          );
        })}
      </div>
    </div>
  );
}
