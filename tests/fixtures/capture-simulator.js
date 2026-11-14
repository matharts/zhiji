// 从仓库根目录通过 playwright-cli run-code 执行，操作步骤见 tests/fixtures/simulator/README.md。
async (page) => {
  const out = 'output/playwright';
  // 按参考模拟器固定的控件顺序定位单选组。
  const group = (i) => page.getByRole('radiogroup').nth(i);
  // 选择后等待真实选中状态，避免截取尚未更新的参考结果。
  const select = async (i, label) => {
    await group(i).getByText(label, { exact: true }).click();
    await page.waitForFunction(([index, text]) => {
      const g = document.querySelectorAll('[role="radiogroup"]')[index];
      const checked = g && g.querySelector('input:checked');
      return checked && checked.closest('label')?.textContent?.trim() === text;
    }, [i, label]);
  };
  // 读取选中参数和实际可见宫位文本，隐藏图层不计入浏览器观察。
  const data = async () => page.evaluate(() => {
    const selected = Array.from(document.querySelectorAll('[role="radiogroup"]')).map(g => {
      const input = g.querySelector('input:checked');
      return input?.closest('label')?.textContent?.trim() ?? null;
    });
    const palaces = Array.from(document.querySelectorAll('#simulator > g > svg')).map(svg => ({
      x: Number(svg.getAttribute('x')), y: Number(svg.getAttribute('y')),
      fill: svg.querySelector('rect')?.getAttribute('fill'),
      visibleText: Array.from(svg.querySelectorAll('text')).filter(el => {
        let current = el;
        while (current && current !== svg) {
          if (getComputedStyle(current).display === 'none' || getComputedStyle(current).visibility === 'hidden') return false;
          current = current.parentElement;
        }
        return !!el.textContent.trim();
      }).map(el => el.textContent.trim())
    }));
    return { selected, palaces, fiveElement: document.querySelector('.ant-tag')?.textContent ?? null };
  });
  await page.reload();
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(2000);
  const initial = await data();
  await page.screenshot({path: `${out}/simulator-initial.png`});
  await page.locator('#simulator').screenshot({path: `${out}/simulator-initial-chart.png`});
  await select(1, '甲');
  const allowed = await group(2).locator('input').evaluateAll(list => list.map(x => !x.disabled));
  await select(2, '子');
  await select(3, '辰');
  await select(4, '正');
  await select(5, '子');
  await page.waitForFunction(() => {
    const groups = document.querySelectorAll('[role="radiogroup"]');
    return groups[6].querySelector('input:checked') && groups[7].querySelector('input:checked');
  });
  const male = await data();
  await page.waitForTimeout(2000);
  await page.screenshot({path: `${out}/simulator-male-jiazi.png`});
  await page.locator('#simulator').screenshot({path: `${out}/simulator-male-jiazi-chart.png`});
  await page.locator('#simulator > g > svg').nth(0).click({position:{x: 75,y:75}});
  const flying = await data();
  await page.waitForTimeout(2000);
  await page.locator('#simulator').screenshot({path: `${out}/simulator-male-flying-chart.png`});
  await page.locator('#simulator > g > svg').nth(0).click({position:{x: 75,y:75}});
  const flyingClear = await data();
  await select(7, '卯');
  await select(8, '辰');
  const yearly = await data();
  await page.waitForTimeout(2000);
  await page.locator('#simulator').screenshot({path: `${out}/simulator-male-yearly-chart.png`});
  let exportResult;
  try {
    const downloadPromise = page.waitForEvent('download', {timeout: 15000});
    await page.getByRole('button', {name: '导出图片'}).click();
    const download = await downloadPromise;
    await download.saveAs(`${out}/simulator-export.png`);
    exportResult = { filename: download.suggestedFilename(), failure: await download.failure() };
  } catch (e) { exportResult = { error: String(e) }; }
  await select(6, '午');
  const manual = await data();
  await page.waitForTimeout(2000);
  await page.locator('#simulator').screenshot({path: `${out}/simulator-manual-main-chart.png`});
  await select(0, '女');
  await select(1, '乙');
  const femaleAfterStem = await data();
  const allowedFemale = await group(2).locator('input').evaluateAll(list => list.map(x => !x.disabled));
  await select(2, '丑');
  await select(3, '酉');
  const female = await data();
  await page.waitForTimeout(2000);
  await page.locator('#simulator').screenshot({path: `${out}/simulator-female-manual-chart.png`});
  await page.getByRole('switch').nth(5).click();
  await page.getByRole('checkbox', {name:'生年忌'}).click();
  await page.waitForFunction(() => [...document.querySelectorAll('label')].some(label => label.textContent.trim() === '生年忌' && label.querySelector('input')?.checked === false));
  const toggles = await data();
  await page.waitForTimeout(2000);
  await page.locator('#simulator').screenshot({path: `${out}/simulator-female-toggles-chart.png`});
  await page.locator('button:has(.anticon-undo)').first().click();
  const yearlyReset = await data();
  await select(0, '男');
  const reverse = await data();
  await page.waitForTimeout(2000);
  await page.locator('#simulator').screenshot({path: `${out}/simulator-male-reverse-chart.png`});
  const colorTrigger = page.locator('.ant-color-picker-trigger').first();
  // 读取浏览器最终计算颜色，用于核验改色和重置的实际效果。
  const colorValue = () => colorTrigger.locator('.ant-color-picker-color-block-inner').evaluate(el => getComputedStyle(el).backgroundColor);
  const defaultColor = await colorValue();
  await colorTrigger.click();
  await page.getByRole('textbox').waitFor({state: 'visible'});
  await page.waitForTimeout(300);
  await page.screenshot({path: `${out}/simulator-color-picker.png`});
  await page.getByRole('textbox').fill('123456');
  await page.getByRole('textbox').press('Enter');
  await page.waitForFunction(() => getComputedStyle(document.querySelector('.ant-color-picker-color-block-inner')).backgroundColor === 'rgb(18, 52, 86)');
  const changedColor = await colorValue();
  await colorTrigger.click();
  await page.locator('button:has(.anticon-undo)').last().click();
  await page.waitForFunction(() => getComputedStyle(document.querySelector('.ant-color-picker-color-block-inner')).backgroundColor === 'rgb(234, 205, 118)');
  const resetColor = await colorValue();
  const color = {defaultColor, changedColor, resetColor};
  const output = {viewport:{width:1300,height:800,dpr:1},initial,allowed,male,flying,flyingClear,yearly,exportResult,manual,femaleAfterStem,allowedFemale,female,toggles,yearlyReset,reverse,color};
  return output;
}
