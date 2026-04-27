// config: indent_width=4
module m; // header
    // leading
    wire x; // trailing
    initial begin
        /* inline */ y = 1;
    end
endmodule
// expected -----
module m; // header
    // leading
    wire x; // trailing
    initial begin
        /* inline */ y = 1;
    end
endmodule
