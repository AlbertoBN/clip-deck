import '@testing-library/jest-dom/vitest'

// jsdom doesn't implement scrollIntoView; stub it so components that call it
// (e.g. keyboard list navigation) don't throw in tests that don't care about
// scrolling. Tests that do care can still override this per-test.
Element.prototype.scrollIntoView = Element.prototype.scrollIntoView ?? (() => {})
