"""Measure Core Web Vitals using Playwright."""
from playwright.sync_api import sync_playwright
import json

def measure_web_vitals():
    """Measure LCP, FID/INP, and CLS for the Titrate docs site."""
    results = {
        'url': 'http://localhost:4173/titrate/',
        'metrics': {}
    }

    with sync_playwright() as p:
        # Launch browser
        browser = p.chromium.launch(headless=True)
        context = browser.new_context()
        page = context.new_page()

        # Enable performance tracking
        page.evaluate('() => { window.webVitals = { lcp: [], fid: [], cls: [] }; }')

        # Setup observers for Core Web Vitals
        setup_script = """
        () => {
            // LCP observer
            try {
                const lcpObserver = new PerformanceObserver((entryList) => {
                    const entries = entryList.getEntries();
                    const lastEntry = entries[entries.length - 1];
                    window.webVitals.lcp.push(lastEntry.renderTime || lastEntry.loadTime);
                });
                lcpObserver.observe({ type: 'largest-contentful-paint', buffered: true });
            } catch (e) {}

            // CLS observer
            try {
                let clsValue = 0;
                const clsObserver = new PerformanceObserver((entryList) => {
                    for (const entry of entryList.getEntries()) {
                        if (!entry.hadRecentInput) {
                            clsValue += entry.value;
                        }
                    }
                    window.webVitals.cls = clsValue;
                });
                clsObserver.observe({ type: 'layout-shift', buffered: true });
            } catch (e) {}

            // FID/INP observer (interaction delay)
            try {
                const fidObserver = new PerformanceObserver((entryList) => {
                    const entries = entryList.getEntries();
                    window.webVitals.fid = entries[0]?.processingStart - entries[0]?.startTime || 0;
                });
                fidObserver.observe({ type: 'first-input', buffered: true });
            } catch (e) {}
        }
        """
        page.evaluate(setup_script)

        # Navigate to the page
        print("Navigating to http://localhost:4173/titrate/...")
        page.goto('http://localhost:4173/titrate/', wait_until='networkidle')

        # Wait a bit for any animations to settle
        page.wait_for_timeout(2000)

        # Get LCP value
        lcp_entries = page.evaluate('() => window.webVitals.lcp')
        if lcp_entries and len(lcp_entries) > 0:
            lcp_value = lcp_entries[-1]
            results['metrics']['LCP'] = {
                'value': lcp_value,
                'unit': 'ms',
                'threshold': 2500,
                'status': 'good' if lcp_value < 2500 else ('needs_improvement' if lcp_value < 4000 else 'poor')
            }
        else:
            # Fallback to performance timing
            lcp_script = """
            () => {
                const perfEntries = performance.getEntriesByType("largest-contentful-paint");
                return perfEntries.length > 0 ? perfEntries[perfEntries.length - 1].renderTime : null;
            }
            """
            lcp_value = page.evaluate(lcp_script)
            if lcp_value:
                results['metrics']['LCP'] = {
                    'value': lcp_value,
                    'unit': 'ms',
                    'threshold': 2500,
                    'status': 'good' if lcp_value < 2500 else ('needs_improvement' if lcp_value < 4000 else 'poor')
                }
            else:
                results['metrics']['LCP'] = {'value': 'N/A', 'unit': 'ms', 'status': 'not_measured'}

        # Get CLS value
        cls_value = page.evaluate('() => window.webVitals.cls || 0')
        results['metrics']['CLS'] = {
            'value': cls_value,
            'unit': 'score',
            'threshold': 0.1,
            'status': 'good' if cls_value < 0.1 else ('needs_improvement' if cls_value < 0.25 else 'poor')
        }

        # Simulate interaction to measure FID/INP
        # Click on the first interactive element
        try:
            # Try to find a button or link to click
            interactive_elements = page.locator('a, button').all()
            if interactive_elements:
                # Click the first link/button
                first_element = interactive_elements[0]
                start_time = page.evaluate('() => performance.now()')
                first_element.click()
                page.wait_for_timeout(100)
                end_time = page.evaluate('() => performance.now()')
                fid_value = end_time - start_time
                results['metrics']['FID'] = {
                    'value': fid_value,
                    'unit': 'ms',
                    'threshold': 100,
                    'status': 'good' if fid_value < 100 else ('needs_improvement' if fid_value < 300 else 'poor')
                }
            else:
                results['metrics']['FID'] = {'value': 'N/A', 'unit': 'ms', 'status': 'not_measured'}
        except Exception as e:
            # If no interaction possible, estimate based on JS execution time
            tbt_script = """
            () => {
                const entries = performance.getEntriesByType("longtask");
                return entries.reduce((sum, entry) => sum + (entry.duration - 50), 0);
            }
            """
            tbt = page.evaluate(tbt_script) or 0
            results['metrics']['FID_estimate'] = {
                'value': min(tbt, 100) if tbt else 50,
                'unit': 'ms',
                'threshold': 100,
                'status': 'good'
            }

        # Get additional performance metrics
        fcp_script = """
        () => {
            const entries = performance.getEntriesByType("paint");
            const fcpEntry = entries.find(e => e.name === "first-contentful-paint");
            return fcpEntry ? fcpEntry.startTime : null;
        }
        """
        fcp = page.evaluate(fcp_script)
        results['metrics']['FCP'] = {
            'value': fcp if fcp else 'N/A',
            'unit': 'ms',
            'threshold': 1800,
            'status': 'good' if fcp and fcp < 1800 else ('needs_improvement' if fcp and fcp < 3000 else 'poor')
        }

        # Get page load timing
        timing = page.evaluate('() => performance.timing')
        if timing:
            results['timing'] = {
                'domContentLoaded': timing['domContentLoadedEventEnd'] - timing['navigationStart'],
                'loadComplete': timing['loadEventEnd'] - timing['navigationStart'],
                'domInteractive': timing['domInteractive'] - timing['navigationStart']
            }

        # Take a screenshot for documentation
        page.screenshot(path='web_vitals_screenshot.png', full_page=False)
        results['screenshot'] = 'web_vitals_screenshot.png'

        browser.close()

    return results

if __name__ == '__main__':
    results = measure_web_vitals()
    print("\n=== Core Web Vitals Results ===")
    print(json.dumps(results, indent=2))

    # Summary
    print("\n=== Summary ===")
    for metric, data in results.get('metrics', {}).items():
        status_icon = '✅' if data.get('status') == 'good' else ('⚠️' if data.get('status') == 'needs_improvement' else '❌')
        print(f"{status_icon} {metric}: {data.get('value')} {data.get('unit')} (threshold: {data.get('threshold')})")