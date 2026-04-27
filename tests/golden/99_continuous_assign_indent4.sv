// config: indent_width=4
module m;
    assign  out  =  a  &  b;
endmodule
// expected -----
module m;
    assign out = a & b;
endmodule
