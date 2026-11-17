// Temporary compatibility for sourcegraph/scip-python#224. Only the invalid
// separator constructor is changed, only while loading this child process's indexer.
const NativeRegExp = global.RegExp;
global.RegExp = new Proxy(NativeRegExp, {
  construct(target, args, newTarget) {
    if (args[0] === '\\' && args[1] === 'g') {
      args = ['[\\\\/]', 'g'];
    }
    return Reflect.construct(target, args, newTarget);
  },
});
try {
  require(process.argv[1]);
} finally {
  global.RegExp = NativeRegExp;
}
