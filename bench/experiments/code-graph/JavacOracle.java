import com.sun.source.tree.*;
import com.sun.source.util.*;
import javax.lang.model.element.*;
import javax.tools.*;
import java.nio.file.*;
import java.util.*;
import java.util.stream.*;

/** javac's own resolution of method calls: file TAB line TAB name TAB target file (project sources only). */
public class JavacOracle {
    public static void main(String[] a) throws Exception {
        Path root = Paths.get(a[0]).toAbsolutePath().normalize();
        List<Path> files;
        try (Stream<Path> s = Files.walk(root)) {
            files = s.filter(p -> p.toString().endsWith(".java") && p.toString().replace('\\', '/').contains("src/main/java")
                    && !p.getFileName().toString().equals("module-info.java")).collect(Collectors.toList());
        }
        JavaCompiler jc = ToolProvider.getSystemJavaCompiler();
        StandardJavaFileManager fm = jc.getStandardFileManager(null, null, null);
        JavacTask task = (JavacTask) jc.getTask(null, fm, d -> {}, List.of("-proc:none", "-Xlint:none"), null, fm.getJavaFileObjectsFromPaths(files));
        Iterable<? extends CompilationUnitTree> units = task.parse();
        task.analyze();
        Trees trees = Trees.instance(task);
        Set<String> mine = files.stream().map(p -> p.toString()).collect(Collectors.toSet());
        StringBuilder out = new StringBuilder();
        for (CompilationUnitTree cu : units) {
            String rel = root.relativize(Paths.get(cu.getSourceFile().toUri())).toString().replace('\\', '/');
            LineMap lm = cu.getLineMap();
            new TreePathScanner<Void, Void>() {
                @Override public Void visitMethodInvocation(MethodInvocationTree t, Void v) {
                    Element e = trees.getElement(getCurrentPath());
                    if (e instanceof ExecutableElement && e.getKind() == ElementKind.METHOD) {
                        TreePath tp = trees.getPath(e);
                        if (tp != null) {
                            String tf = root.relativize(Paths.get(tp.getCompilationUnit().getSourceFile().toUri())).toString().replace('\\', '/');
                            long pos = trees.getSourcePositions().getStartPosition(cu, t.getMethodSelect() instanceof MemberSelectTree m ? m : t);
                            // the parser reports the line of the argument list
                            long ap = trees.getSourcePositions().getEndPosition(cu, t.getMethodSelect());
                            out.append(rel).append('\t').append(lm.getLineNumber(Math.max(ap, 0))).append('\t')
                               .append(e.getSimpleName()).append('\t').append(tf).append('\n');
                        }
                    }
                    return super.visitMethodInvocation(t, v);
                }
            }.scan(cu, null);
        }
        System.out.print(out);
    }
}
