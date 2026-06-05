self.onmessage = async (event: MessageEvent<{ nonce: string; difficulty: number }>) => {
  const { nonce, difficulty } = event.data;
  const encoder = new TextEncoder();
  let solution = 0;
  while (true) {
    const input = `${nonce}:${solution}`;
    const hash = new Uint8Array(await crypto.subtle.digest('SHA-256', encoder.encode(input)));
    let zeroBits = 0;
    for (let i = 0; i < hash.length; i++) {
      if (hash[i] === 0) {
        zeroBits += 8;
      } else {
        let b = hash[i];
        while ((b & 0x80) === 0) {
          zeroBits++;
          b <<= 1;
        }
        break;
      }
    }
    if (zeroBits >= difficulty) {
      self.postMessage({ solution });
      return;
    }
    solution++;
  }
};
