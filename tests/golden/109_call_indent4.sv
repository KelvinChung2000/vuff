// config: indent_width=4
module m;
    initial begin
        $display ("x=%d" , x);
        foo (a , b);
    end
endmodule
// expected -----
module m;
    initial begin
        $display("x=%d", x);
        foo(a, b);
    end
endmodule
