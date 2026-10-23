// jsdom lacks these; the app only needs them to exist.
if (!window.matchMedia) {
  window.matchMedia = (query: string) =>
    ({ matches: false, media: query, addEventListener() {}, removeEventListener() {} }) as unknown as MediaQueryList;
}

import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

afterEach(cleanup);
