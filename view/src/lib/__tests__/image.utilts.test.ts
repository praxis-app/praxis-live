import { validateImageInput } from '@/lib/image.utilts';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@/lib/shared.utils', () => ({
  t: (key: string) => key,
}));

const image = (name: string) =>
  new File(['image'], name, { type: 'image/png' });

describe('validateImageInput', () => {
  it('allows four image attachments', () => {
    expect(() =>
      validateImageInput([
        image('one.png'),
        image('two.png'),
        image('three.png'),
        image('four.png'),
      ]),
    ).not.toThrow();
  });

  it('rejects a fifth image attachment', () => {
    expect(() =>
      validateImageInput([
        image('one.png'),
        image('two.png'),
        image('three.png'),
        image('four.png'),
        image('five.png'),
      ]),
    ).toThrow('images.errors.tooManyImages');
  });
});
