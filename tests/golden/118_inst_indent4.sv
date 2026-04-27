// config: indent_width=4
module top;
    sub  u1  (.clk(clk), .rst(rst));
    fifo #(.W(8))  u2  (.in(in), .out(out));
endmodule
// expected -----
module top;
    sub u1 (.clk(clk), .rst(rst));
    fifo #(.W(8)) u2 (.in(in), .out(out));
endmodule
