// config: indent_width=4
module m;
    function int   double(input int v);
        return v * 2;
    endfunction
endmodule
// expected -----
module m;
    function int double(input int v);
        return v * 2;
    endfunction
endmodule
